import { createAction } from '@reduxjs/toolkit';
import { Reset, ComponentAdd, ComponentRemove, StateChange } from '../../api/registry';

export const reset = createAction<Reset>('registry/reset');
export const componentAdd = createAction<ComponentAdd>('registry/component-add');
export const componentRemove = createAction<ComponentRemove>('registry/component-remove');
export const attributeChange = createAction<StateChange>('registry/attribute-change');
