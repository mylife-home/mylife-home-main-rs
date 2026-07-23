import { Component, Plugin } from './component-model';
// direct import to avoid require all common in ui
// import { InstanceInfo } from 'mylife-home-common/dist/instance-info/types';

// export { InstanceInfo };

export interface InstanceInfo {
  /**
   * 'ui' | 'studio' | 'core' | 'driver? (for arduino/esp/...)'
   */
  type: string;

  /**
   * main: Raspberry ... | nodemcu | x64
   * others are details like ram, cpu, ...
   */
  hardware: { [name: string]: string };
  /**
   * --- rpi
   * os: linux-xxx
   * node: 24.5
   * mylife-home-core: 1.0.0
   * mylife-home-common: 1.0.0
   * --- esp/arduino
   * mylife: 1.21.4
   */
  versions: {
    [component: string]: string;
  };

  systemUptime: number;
  instanceUptime: number;
  hostname: string;
  capabilities: string[];

  wifi?: {
    rssi: number;
  }
}


export interface UpdateInstanceInfoData {
  operation: 'set' | 'clear';
  instanceName: string;
  data?: InstanceInfo;
}

export interface State {
  component: string;
  name: string;
  value: any;
}

export interface UpdateComponentData {
  operation: 'set' | 'clear';
  instanceName: string;
  type: 'plugin' | 'component' | 'state';
}

export interface ClearData extends UpdateComponentData {
  operation: 'clear';
  id: string;
}

export interface SetComponentData extends UpdateComponentData {
  operation: 'set';
  type: 'component';
  data: Component;
}

export interface SetPluginData extends UpdateComponentData {
  operation: 'set';
  type: 'plugin';
  data: Plugin;
}

export interface SetStateData extends UpdateComponentData {
  operation: 'set';
  type: 'state';
  data: State;
}

export interface HistoryRecord {
  timestamp: number;
  type: 'instance-set' | 'instance-clear' | 'component-set' | 'component-clear' | 'state-set';
}

export interface InstanceHistoryRecord extends HistoryRecord {
  type: 'instance-set' | 'instance-clear';
  instanceName: string;
}

export interface ComponentSetHistoryRecord extends HistoryRecord {
  type: 'component-set';
  instanceName: string;
  componentId: string;
  states?: { [name: string]: any; };
}

export interface ComponentClearHistoryRecord extends HistoryRecord {
  type: 'component-clear';
  instanceName: string;
  componentId: string;
}

export interface StateHistoryRecord extends HistoryRecord {
  type: 'state-set';
  instanceName: string;
  componentId: string;
  stateName: string;
  stateValue: any;
}

export interface Status {
  transportConnected: boolean;
}