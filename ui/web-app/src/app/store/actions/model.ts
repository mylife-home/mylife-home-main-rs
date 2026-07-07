import { createAction } from '@reduxjs/toolkit';
import { AppThunkAction } from '../types';
import { CSS_SET } from '../types/css';
import { NetModel } from '../types/model';
import { resourceQuery } from './resources';
import { viewInit } from './view';

export const modelSet = createAction<NetModel>('model/set');
const cssSet = createAction<string>(CSS_SET);

export const modelInit = (modelHash: string): AppThunkAction => (dispatch, getState) => {
  console.log('modelInit with modelHash', modelHash); // eslint-disable-line no-console

  dispatch(resourceQuery({
    resource: modelHash, 
    onContent: (content: any) => {
      const model = content as NetModel;
      console.log('using model', model);

      dispatch(cssSet(model.styleHash));
      dispatch(modelSet(model));
      dispatch(viewInit());
    }
  }));
};
