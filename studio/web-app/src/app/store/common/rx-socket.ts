import { BehaviorSubject, from, Subject, asyncScheduler } from 'rxjs';
import { filter, map, observeOn } from 'rxjs/operators';
import type { ServiceRequest, ServerMessage, ServiceResponse, Notification } from '../../api/protocol';

const TIMEOUT = 60000;
const RECONNECT_DELAY = 1000;

export interface RequestEvent {
  type: 'begin' | 'end';
  id: string;
}

export interface BeginRequestEvent extends RequestEvent {
  type: 'begin';
  service: string;
}

export interface EndRequestEvent extends RequestEvent {
  type: 'end';
}

class ServerError extends Error {
  public readonly serverType: string;
  public readonly serverStack: string;

  constructor(serverError: { type: string; message: string; stack: string; }) {
    super(`An error occured server-side: ${serverError.message}`);
    this.serverType = serverError.type;
    this.serverStack = serverError.stack;
  }
}

class IdGenerator {
  private counter = 0;

  generate() {
    return `${++this.counter}`;
  }
}

function makeWebSocketUrl(path: string) {
  const link = document.createElement('a');
  link.href = path;
  const absoluteUrl = link.href;
  return absoluteUrl.replace(/^http/, 'ws');
}

export class RxSocket {
  private readonly online$ = new BehaviorSubject<boolean>(false);
  private readonly message$ = new Subject<ServerMessage>();
  private readonly request$ = new Subject<RequestEvent>();
  private readonly requestIdGenerator = new IdGenerator();
  private socket: WebSocket | null = null;
  private reconnectTimer: number | null = null;

  constructor() {
    this.connect();
  }

  private connect() {
    if (this.socket && (this.socket.readyState === WebSocket.OPEN || this.socket.readyState === WebSocket.CONNECTING)) {
      return;
    }

    const socket = new WebSocket(makeWebSocketUrl('websocket'));
    this.socket = socket;

    socket.addEventListener('open', () => {
      this.online$.next(true);
      if (this.reconnectTimer !== null) {
        clearTimeout(this.reconnectTimer);
        this.reconnectTimer = null;
      }
    });

    socket.addEventListener('close', () => {
      this.online$.next(false);
      this.scheduleReconnect();
    });

    socket.addEventListener('error', () => {
      this.online$.next(false);
    });

    socket.addEventListener('message', (event: MessageEvent) => {
      const raw = typeof event.data === 'string' ? event.data : '';
      if (!raw) {
        return;
      }

      try {
        const message = JSON.parse(raw) as ServerMessage;
        this.message$.next(message);
      } catch (error) {
        console.error('Failed to parse websocket message', error);
      }
    });
  }

  private scheduleReconnect() {
    if (this.reconnectTimer !== null) {
      return;
    }

    this.reconnectTimer = window.setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, RECONNECT_DELAY);
  }

  online() {
    return this.online$.asObservable();
  }

  notifications() {
    return this.message$.pipe(
      observeOn(asyncScheduler),
      filter((msg) => msg.type === 'notification'),
      map(msg => msg as Notification)
    );
  }

  request() {
    return this.request$.asObservable();
  }

  call(service: string, payload: any) {
    return from(this.wrapServiceCall(service, payload));
  }

  private async wrapServiceCall(service: string, payload: any) {
    const id = this.requestIdGenerator.generate();

    this.request$.next({ type: 'begin', id, service } as BeginRequestEvent);
    try {
      return await this.serviceCall(id, service, payload);
    } finally {
      this.request$.next({ type: 'end', id } as EndRequestEvent);
    }
  }

  private async serviceCall(requestId: string, service: string, payload: any) {
    return new Promise<any>((resolve, reject) => {
      const socket = this.socket;
      if (!socket || socket.readyState !== WebSocket.OPEN) {
        return reject(new Error(`Cannot send request while disconnected: (service='${service}')`));
      }

      const messageSubscription = this.message$.subscribe((message) => {
        if (message.type !== 'service-response') {
          return;
        }

        const serviceResponse = message as ServiceResponse;
        if (serviceResponse.requestId !== requestId) {
          return;
        }

        cleanup();

        const { error, result } = serviceResponse;
        if (error) {
          return reject(new ServerError(error));
        }

        resolve(result);
      });

      const onlineSubscription = this.online$.subscribe((online) => {
        if (online) {
          return;
        }

        cleanup();
        reject(new Error(`Disconnection while waiting response: (service='${service}')`));
      });

      const onTimeout = () => {
        cleanup();
        reject(new Error(`Request timeout after ${TIMEOUT / 1000} seconds: (service='${service}')`));
      };

      const cleanup = () => {
        messageSubscription.unsubscribe();
        onlineSubscription.unsubscribe();
        clearTimeout(timeout);
      };

      const request: ServiceRequest = { requestId, service, payload };
      socket.send(JSON.stringify(request));

      const timeout = setTimeout(onTimeout, TIMEOUT);
    });
  }
}

export const socket = new RxSocket();
