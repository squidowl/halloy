use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use iced::Subscription;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AllocAnyThread as _, DeclaredClass, define_class, msg_send, sel};
use objc2_app_kit::{
    NSWorkspace, NSWorkspaceDidWakeNotification,
    NSWorkspaceWillSleepNotification,
};
use objc2_foundation::{
    NSActivityOptions, NSObject, NSObjectProtocol, NSProcessInfo, ns_string,
};

use super::Event;

/// Keeps networking active while allowing idle system sleep.
pub(super) struct Activity {
    process_info: Retained<NSProcessInfo>,
    activity: Option<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

impl Activity {
    pub(super) fn new(active: bool) -> Self {
        let mut activity = Self {
            process_info: NSProcessInfo::processInfo(),
            activity: None,
        };
        activity.set_active(active);
        activity
    }

    pub(super) fn set_active(&mut self, active: bool) {
        match (active, self.activity.is_some()) {
            (true, false) => {
                self.activity =
                    Some(self.process_info.beginActivityWithOptions_reason(
                        NSActivityOptions::UserInitiatedAllowingIdleSystemSleep,
                        ns_string!("Maintain IRC connections"),
                    ));
            }
            (false, true) => {
                let activity = self.activity.take().expect("activity exists");

                // SAFETY: This activity was returned by beginActivityWithOptions:reason:.
                unsafe { self.process_info.endActivity(&activity) };
            }
            _ => {}
        }
    }
}

impl Drop for Activity {
    fn drop(&mut self) {
        self.set_active(false);
    }
}

struct Ivars {
    sender: mpsc::UnboundedSender<Event>,
}

define_class! {
    // SAFETY: NSObject has no subclassing requirements and the ivars are Send.
    #[unsafe(super(NSObject))]
    #[name = "HalloyPowerObserver"]
    // An instance variable (ivar) is a variable that exists and holds its
    // value for the life of the NSObject.
    #[ivars = Ivars]
    struct Observer;

    impl Observer {
        #[unsafe(method(systemWillSleep))]
        fn system_will_sleep(&self) {
            let _ = self.ivars().sender.unbounded_send(Event::Suspending);
        }

        #[unsafe(method(systemDidWake))]
        fn system_did_wake(&self) {
            let _ = self.ivars().sender.unbounded_send(Event::Resumed);
        }
    }
}

impl Observer {
    fn new(sender: mpsc::UnboundedSender<Event>) -> Retained<Self> {
        let observer = Self::alloc().set_ivars(Ivars { sender });

        // SAFETY: The observer is allocated and its ivars have been initialized.
        unsafe { msg_send![super(observer), init] }
    }
}

struct Registration(Retained<Observer>);

impl Registration {
    fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        let observer = Observer::new(sender);
        // Workspace associated with the process, shared by all threads
        // of the app.
        let notification_center =
            NSWorkspace::sharedWorkspace().notificationCenter();

        // SAFETY: The selectors are implemented by Observer and it remains
        // alive until both registrations are removed.
        unsafe {
            notification_center.addObserver_selector_name_object(
                &observer,
                sel!(systemWillSleep),
                Some(NSWorkspaceWillSleepNotification),
                None,
            );
            notification_center.addObserver_selector_name_object(
                &observer,
                sel!(systemDidWake),
                Some(NSWorkspaceDidWakeNotification),
                None,
            );
        }

        Self(observer)
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        let notification_center =
            NSWorkspace::sharedWorkspace().notificationCenter();

        // SAFETY: This observer was registered with this notification center in
        // Registration::new.
        unsafe { notification_center.removeObserver(&self.0) };
    }
}

pub fn events() -> Subscription<Event> {
    Subscription::run(|| {
        iced::stream::channel(10, async |mut output| {
            let (sender, mut receiver) = mpsc::unbounded();
            let _registration = Registration::new(sender);

            while let Some(event) = receiver.next().await {
                if output.send(event).await.is_err() {
                    break;
                }
            }
        })
    })
}
