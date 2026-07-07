import { createReducer } from '@reduxjs/toolkit';
import { Model } from '../types/model';
import { modelSet } from '../actions/model';

const DEFAULT: Model = { defaultWindow: {}, windows: {}, controls: {} };

export default createReducer(DEFAULT, (builder) => {
    builder.addCase(modelSet, (state, action) => {
      const model: Model = {
        defaultWindow: action.payload.defaultWindow,
        windows: {},
        controls: {},
      }

      for (const window of action.payload.windows) {
        model.windows[window.id] = window;

        for (const control of window.controls) {
          model.controls[`${window.id}$${control.id}`] = control;
        }
      }

      return model;
    })
});
