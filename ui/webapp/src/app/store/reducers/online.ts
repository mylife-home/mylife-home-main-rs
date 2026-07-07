import { createReducer } from '@reduxjs/toolkit';
import { onlineSet } from '../actions/online';

export default createReducer(false, (builder) => {
  builder.addCase(onlineSet, (state, action) => action.payload);
});
