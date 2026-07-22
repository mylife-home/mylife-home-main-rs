// direct import to avoid require all common in ui
// export { NetComponent as Component, NetPlugin as Plugin, NetMember as Member } from 'mylife-home-common/dist/components/metadata/net';
export interface Component {
    readonly id: string;
    readonly plugin: string;
}
export interface Member {
    readonly description: string;
    readonly memberType: MemberType;
    readonly valueType: string;
}
export interface Plugin {
    readonly name: string;
    readonly module: string;
    readonly usage: PluginUsage;
    readonly version: string;
    readonly description: string;
    readonly members: {
        [name: string]: Member;
    };
    readonly config: {
        [name: string]: ConfigItem;
    };
}

// export { PluginUsage, MemberType, ConfigItem, ConfigType } from 'mylife-home-common/dist/components/metadata/plugin';

export declare const enum MemberType {
    ACTION = "action",
    STATE = "state"
}

export declare const enum PluginUsage {
    SENSOR = "sensor",
    ACTUATOR = "actuator",
    LOGIC = "logic",
    UI = "ui"
}

export interface ConfigItem {
    readonly description: string;
    readonly valueType: ConfigType;
}

export declare const enum ConfigType {
    STRING = "string",
    BOOL = "bool",
    INTEGER = "integer",
    FLOAT = "float"
}