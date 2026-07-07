import { createReducer, PayloadAction } from '@reduxjs/toolkit';
import { viewPopup, internalViewClose, internalViewChange } from '../actions/view';

const DEFAULT: string[] = [];

export default createReducer(DEFAULT, (builder) => {
  builder
  .addCase(viewPopup, (state, action: PayloadAction<string>) => [...state, action.payload])
  .addCase(internalViewClose, (state, action) => pop(state))
  .addCase(internalViewChange, (state, action: PayloadAction<string>) => [action.payload])
});

function pop<T>(array: T[]): T[] {
  return [...array.slice(0, array.length - 1)];
}
