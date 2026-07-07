import { Middleware } from 'redux';
import { PayloadAction } from '@reduxjs/toolkit';
import { ActionComponent } from '../../api/model';
import { SocketMessage, ActionMessage } from '../../api/socket';
import { ACTION_COMPONENT } from '../types/actions';
import { onlineSet } from '../actions/online';
import { reset, componentAdd, componentRemove, attributeChange } from '../actions/registry';
import { modelInit } from '../actions/model';

const PING_INTERVAL = 500;         // Send ping every 0.5s
const IDLE_TIMEOUT = 1000;         // If no messages for 1s → reconnect
const BASE_RECONNECT_DELAY = 500;  // Start retry after 0.5s
const MAX_RECONNECT_DELAY = 10000; // Cap retries at 10s

let isBackground = false;

document.addEventListener("visibilitychange", () => {
  isBackground = document.hidden;
});

export const socketMiddleware: Middleware = (store) => (next) => {
  // Note: do not use 'ws' if you want to use webpack.devServer.
  const url = makeWebSocketUrl('websocket');
  const socket = new ReconnectingWebSocket(url);

  socket.onopen = () => {
    next(onlineSet(true));
  };

  socket.onclose = () => next(onlineSet(false));

  socket.onmessage = (type, data) => {
    switch (type) {
      case 'state':
        next(reset(data));
        break;

      case 'add':
        next(componentAdd(data));
        break;

      case 'remove':
        next(componentRemove(data));
        break;

      case 'change':
        next(attributeChange(data));
        break;

      case 'modelHash':
        next(modelInit(data) as any); // TODO: proper cast: AppThunkAction => AnyAction
        break;
    }
  };

  return (action: any) => {
    if (action.type === ACTION_COMPONENT) {
      const typedAction = action as PayloadAction<ActionComponent>;
      socket.send('action', typedAction.payload as ActionMessage);
    }

    return next(action);
  };
};

// Build a WebSocket URL that follows browser relative resolution rules
function makeWebSocketUrl(path: string) {
  // Create a <a> element to let the browser resolve the relative path
  const link = document.createElement('a');
  link.href = path;
  const absoluteUrl = link.href;

  // Replace the protocol only (http → ws, https → wss)
  return absoluteUrl.replace(/^http/, 'ws');
}

type Timer = ReturnType<typeof setTimeout> | ReturnType<typeof setInterval>;
type SocketMessageType = SocketMessage['type'];

class ReconnectingWebSocket {
  private url: string;
  private ws: WebSocket | null = null;
  private closeCalled: boolean = false;
  private pingInterval: Timer | null = null;
  private idleTimeout: Timer | null = null;
  private reconnectDelay: number = BASE_RECONNECT_DELAY;

  // User callbacks
  public onopen: () => void = () => {};
  public onclose: () => void = () => {};
  public onmessage: (type: SocketMessageType, data: any) => void = () => {};

  constructor(url: string) {
    this.url = url;
    this.connect();
  }

  private connect() {
    this.ws = new WebSocket(this.url);
    this.closeCalled = false;

    this.ws.onopen = () => {
      console.log("Connected to", this.url);

      // Reset backoff
      this.reconnectDelay = BASE_RECONNECT_DELAY;

      // Start ping loop
      this.pingInterval = setInterval(() => {
        if (this.ws?.readyState === WebSocket.OPEN) {
          this.send("ping", null);
        }
      }, PING_INTERVAL);

      this.resetIdleTimer();
      this.onopen();
    };

    this.ws.onmessage = (event: MessageEvent) => {
      this.resetIdleTimer();

      try {
        const { type, data } = JSON.parse(event.data) as SocketMessage;
        this.onmessage(type, data);
      } catch (error) {
        console.error("Error handling WebSocket message:", error);
      }
    };

    this.ws.onclose = () => {
      this.wsClose();
    };

    this.ws.onerror = (err: Event) => {
      console.error("WebSocket error:", err);
      this.wsClose();
    };
  }

  private resetIdleTimer() {
    clearTimeout(this.idleTimeout);

    this.idleTimeout = setTimeout(() => {
      if (isBackground) {
        console.warn("Idle timeout while background, ignoring");
        this.resetIdleTimer();
      } else {
        console.warn("Idle timeout, forcing reconnect...");
        this.wsClose();
      }
    }, IDLE_TIMEOUT);
  }

  private wsClose() {
    if (this.closeCalled) {
      return;
    }

    this.closeCalled = true;

    this.ws?.close();
    this.ws = null;

    console.warn("Connection closed, retrying in", this.reconnectDelay, "ms");
    this.cleanup();
    setTimeout(() => this.connect(), this.reconnectDelay);

    // Exponential backoff
    this.reconnectDelay = Math.min(this.reconnectDelay * 2, MAX_RECONNECT_DELAY);

    this.onclose();
  }

  private cleanup() {
    clearInterval(this.pingInterval);
    clearTimeout(this.idleTimeout);
  }

  public send(type: SocketMessageType, data: any) {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      const message: SocketMessage = { type, data };
      this.ws.send(JSON.stringify(message));
    } else {
      console.warn("Cannot send, socket not open");
    }
  }
}
