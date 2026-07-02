# mylife-home-main-rs
MyLife Home Main, Rust implementation

## Notes old

### model

registry + components = 1 actor
state change => post to the registry mailbox (sync or async is same -> enqueue)

bus/network = 1 actor

add extensions as mailbox handlers, which can react to kind of messages
add custom kind of messages per extension (message is a trait)

### components

Async component : handler sync to implement if needed in the plugin, which take &mut self (like actions, so can update state)
Then each plugin can have a MessageSender instance, which can be used in another async task to "call" the handler from within the components task (like setImmediate), with an arg if possible

## TODO

core:
- component manager: store + component list
- bindings
- plugins
