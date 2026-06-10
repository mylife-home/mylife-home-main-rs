use std::collections::HashSet;

use crate::{
    bus::{
        BusData, BusMessage,
        client::{MqttEvent, TopicBuilder},
        encoding,
    },
    utils::{
        mailbox::MailboxHandle,
        observable::{EventType, Observable, Subject},
    },
};

use super::BusHandler;

pub struct PresenceEventType;

impl EventType for PresenceEventType {
    type Event<'a> = PresenceEvent<'a>;
}

/// PresenceEvent represents the presence status of an instance, emitted when an instance goes online or offline.
pub enum PresenceEvent<'a> {
    /// InstanceOnline is emitted when an instance goes online, containing the instance name.
    InstanceOnline { instance_name: &'a str },

    /// InstanceOffline is emitted when an instance goes offline, containing the instance name.
    InstanceOffline { instance_name: &'a str },
}

/// Presence tracks the online status of instances and emits events when they change.
pub struct Presence {
    instances: HashSet<String>,
    subject: Subject<PresenceEventType>,
}

impl Presence {
    pub fn new() -> Self {
        Self {
            instances: HashSet::new(),
            subject: Subject::new(),
        }
    }

    /// Marks an instance as online, emitting an InstanceOnline event if it was not already online.
    pub fn is_online(&self, instance_name: &str) -> bool {
        self.instances.contains(instance_name)
    }

    /// Get the list of online instances.
    pub fn get_online_instances(&self) -> Vec<&String> {
        self.instances.iter().collect()
    }

    /// Update the online status of an instance, emitting an event if it changes.
    ///
    /// Note: reserved for PresenceHandler
    fn update_instance_status(&mut self, instance_name: &str, online: bool) {
        if online {
            if self.instances.insert(instance_name.to_string()) {
                log::debug!("instance {} is now online", instance_name);
                self.subject
                    .notify(&PresenceEvent::InstanceOnline { instance_name });
            }
        } else {
            if self.instances.remove(instance_name) {
                log::debug!("instance {} is now offline", instance_name);
                self.subject
                    .notify(&PresenceEvent::InstanceOffline { instance_name });
            }
        }
    }

    /// Clear all presence status, marking all instances as offline and emitting events for each.
    ///
    /// Note: reserved for PresenceHandler
    fn clear_status(&mut self) {
        self.instances.clear();
    }
}

impl Observable<PresenceEventType> for Presence {
    fn observe(
        &self,
        observer: Box<crate::utils::observable::Observer<PresenceEventType>>,
    ) -> crate::utils::observable::ObserverId {
        self.subject.observe(observer)
    }

    fn unobserve(&self, id: crate::utils::observable::ObserverId) -> bool {
        self.subject.unobserve(id)
    }
}

pub struct PresenceHandler;

impl BusHandler for PresenceHandler {
    fn init(&mut self, data: &mut BusData, _: &MailboxHandle<Box<dyn BusMessage>>) {
        data.client_mut()
            .subscribe(TopicBuilder::any_instance("online").build());
    }

    fn handle_mqtt(&mut self, data: &mut BusData, event: &MqttEvent) {
        match event {
            MqttEvent::Disconnected { .. } => {
                data.presence.clear_status();
            }

            MqttEvent::Message(message) => {
                let (Some(domain), Some(instance_name)) = (message.domain(), message.instance())
                else {
                    return;
                };

                if domain != "online" || instance_name == data.client().instance_name() {
                    return;
                }

                let online = match encoding::read_bool(message.payload()) {
                    Ok(value) => value,
                    Err(e) => {
                        log::error!(
                            "Error reading online value ({:?}): {}",
                            message.payload(),
                            e
                        );
                        return;
                    }
                };

                data.presence_mut()
                    .update_instance_status(instance_name, online);
            }

            _ => {}
        }
    }
}
