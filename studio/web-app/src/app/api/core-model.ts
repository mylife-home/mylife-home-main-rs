// direct import to avoid require all common in core
// we depend on mylife-home-core only for this
// export * from 'mylife-home-core/dist/store/model';

export const enum StoreItemType {
  COMPONENT = 'component',
  BINDING = 'binding'
}

export interface StoreItem {
  readonly type: StoreItemType;
  readonly config: ComponentConfig | BindingConfig;
}

export interface ComponentConfig {
  readonly id: string;
  readonly plugin: string;
  readonly config: { [name: string]: any };
}

export interface BindingConfig {
  readonly sourceComponent: string;
  readonly sourceState: string;
  readonly targetComponent: string;
  readonly targetAction: string;
}

