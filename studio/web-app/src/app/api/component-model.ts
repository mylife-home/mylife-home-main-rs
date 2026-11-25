import type { ConfigType as ConfigTypeUnion, MemberType as MemberTypeUnion, PluginUsage as PluginUsageUnion } from './generated/component-model';

export * from './generated/component-model';

/**
 * Every variant of a union, keyed by its own name in upper case. A variant
 * added or dropped on the Rust side stops this file compiling.
 */
type Variants<Variant extends string> = { [Key in Uppercase<Variant>]: Lowercase<Key> };

/**
 * ts-rs emits types and nothing else, so a union the app picks from at runtime
 * gets its constants here. Each name carries both meanings, as a union and as
 * the object of its variants, so one import serves a type annotation and a
 * comparison alike.
 */

export type MemberType = MemberTypeUnion;
export const MemberType = { STATE: 'state', ACTION: 'action' } as const satisfies Variants<MemberTypeUnion>;

export type ConfigType = ConfigTypeUnion;
export const ConfigType = { STRING: 'string', BOOL: 'bool', INTEGER: 'integer', FLOAT: 'float' } as const satisfies Variants<ConfigTypeUnion>;

export type PluginUsage = PluginUsageUnion;
export const PluginUsage = { SENSOR: 'sensor', ACTUATOR: 'actuator', LOGIC: 'logic', UI: 'ui' } as const satisfies Variants<PluginUsageUnion>;
