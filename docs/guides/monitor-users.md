# Monitor Users

Halloy has [monitor](https://ircv3.net/specs/extensions/monitor) support if the server has the IRCv3 Monitor extension (a protocol for notification of when clients become online/offline).

To use the feature you need to add the user(s) you wish to monitor. This can be done in two ways:

* You can add a list of user directly to the configuration file. [See configuration option.](/configuration/servers#monitor)
* You can add users through `/monitor` directly in Halloy (users added this way will not be remembered across application restarts).

Examples with the `/monitor` command:

```
/monitor + casperstorm # Add user to list being monitored
/monitor - casperstorm # Remove user from list being monitored
/monitor c # Clear the list of users being monitored
/monitor l # Get list of users being monitored
/monitor s # For each user in the list being monitored, get their current status
```

[Notifications](/configuration/notifications#types) can be configured for when monitored users come online or go offline.  For example:

```toml
[notifications.monitored_online]
show_toast = true

[notifications.monitored_offline]
show_toast = true
```

will show a toast notification whenever a monitored user changes between offline and online.
