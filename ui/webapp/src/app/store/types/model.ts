import { Window, DefaultWindow, Control } from '../../api/model';
export type { Model as NetModel, Window, Control, ControlDisplay, ControlDisplayMapItem, ControlText, ControlTextContextItem, Action, ActionComponent, ActionWindow, Resource } from '../../api/model';

export interface Model {
  defaultWindow: DefaultWindow;
  windows: { [id: string]: Window };
  controls: { [id: string]: Control };
}