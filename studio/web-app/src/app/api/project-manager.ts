import type { ControlDisplay, ControlDisplayMapItem } from './generated/model';
import type { UiElementPathNode } from './generated/project-manager';

export * from './generated/project-manager';

export * as coreImportData from './generated/project-manager-core-import-data';
export * as coreValidation from './generated/project-manager-core-validation';

/**
 * The ui project stores model types as they are, under names of its own.
 */
export type UiControlDisplayData = ControlDisplay;
export type UiControlDisplayMapItemData = ControlDisplayMapItem;

/**
 * The path to an element inside a ui project, from the window down.
 */
export type UiElementPath = UiElementPathNode[];
