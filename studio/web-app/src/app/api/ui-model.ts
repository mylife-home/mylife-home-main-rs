// direct import to avoid require all common in ui
// we depend on mylife-home-ui only for this
// export * from 'mylife-home-ui/dist/shared/model';
// FIXME: regenerate it, no cross imports
export * from '../../../../../ui/web-app/src/app/api/model';

// export * from 'mylife-home-ui/dist/src/model/definition';

import { Window, DefaultWindow } from '../../../../../ui/web-app/src/app/api/model';

export interface Definition {
  readonly resources: DefinitionResource[];
  readonly styles: DefinitionStyle[];
  readonly windows: Window[];
  readonly defaultWindow: DefaultWindow;
}

export interface DefinitionResource {
  readonly id: string;
  readonly mime: string;
  readonly data: string;
}

export interface DefinitionStyle {
  readonly id: string;
  readonly properties: object;
}
