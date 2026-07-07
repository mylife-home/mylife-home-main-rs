import { createReducer, PayloadAction } from '@reduxjs/toolkit';
import { Reset, ComponentAdd, ComponentRemove, StateChange } from '../../api/registry';
import { RepositoryState } from '../types/registry';
import { reset, componentAdd, componentRemove, attributeChange} from '../actions/registry';

const DEFAULT: RepositoryState = {};

export default createReducer(DEFAULT, (builder) => {
  builder
  .addCase(reset, (state, action: PayloadAction<Reset>) => action.payload)
  .addCase(componentAdd, (state, action: PayloadAction<ComponentAdd>) => ({ ...state, [action.payload.id]: action.payload.attributes }))
  .addCase(componentRemove, (state, action: PayloadAction<ComponentRemove>) => deleteObjectKey(state, action.payload.id))
  .addCase(attributeChange, (state, action: PayloadAction<StateChange>) => ({
    ...state,
    [action.payload.id]: {
      ...state[action.payload.id],
      [action.payload.name]: action.payload.value
    }
  }));
});

function deleteObjectKey<T>(obj: {[id: string]: T}, key: string) :  {[id: string]: T} {
  const { [key]: removed, ...others} = obj;
  void removed;
  return others;
}