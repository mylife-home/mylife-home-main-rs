import { AppState } from '../types';
import { hasWindow } from './model';

export const getView = (state: AppState) => state.view;
export const isViewPopup = (state: AppState) => getView(state).length > 1;

export const isReady = (state: AppState) => {
  const viewId = getView(state)[0];
  return !!viewId && hasWindow(state, viewId);
};
