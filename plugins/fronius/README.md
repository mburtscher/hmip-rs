# Fronius Plugin for HomematicIP

Shows your **Fronius solar system** in your Homematic IP system: solar production, battery, grid connection and house consumption appear as devices on your Home Control Unit (HCU) and in the Homematic IP smartphone app, and can be used in automations, dashboards and energy views.

The plugin talks to your Fronius **SYMO GEN24** inverter over your local network using Fronius' *Solar API*. It does not use any cloud service; nothing leaves your home network.

> **Status:** under development, not production ready. This project is independent and not associated with, endorsed by, or sponsored by Homematic / eQ-3 AG or Fronius.

## What you get

After setup, four devices are available:

| Device | Shows |
|---|---|
| **Wechselrichter** (inverter) | current solar production |
| **Batterie** (battery) | charge level in percent and current battery power |
| **Netzzugang** (grid connection) | power currently drawn from or fed into the grid |
| **Hausverbrauch** (house consumption) | current power consumption of your house |

Values are refreshed **every 10 seconds**.

All power values are reported in **watts** and follow the Homematic IP convention *positive = consuming, negative = supplying*:

| Device | Positive value means | Negative value means |
|---|---|---|
| Wechselrichter | – | the solar panels are producing power (the more negative, the more production) |
| Batterie | the battery is charging | the battery is discharging |
| Netzzugang | power is drawn from the grid | power is fed into the grid |
| Hausverbrauch | the house is consuming power (always ≥ 0) | – |

If your system has no battery or no power meter, the Solar API does not report the matching values and these devices stay empty.

## Requirements

- A Fronius **SYMO GEN24** inverter (or another Fronius inverter that offers the Solar API v1 call `GetPowerFlowRealtimeData`), connected to your home network.
- The **Solar API enabled** in the inverter's web interface (look for the Solar API setting under *Communication* in the inverter's settings; Fronius' documentation describes the exact steps).
- The inverter's **IPv4 address**. Give the inverter a fixed address (or a DHCP reservation in your router), so it does not change.
- A Homematic IP **Home Control Unit** with firmware 1.4.7 or later.

You can check that the Solar API works by opening this address in a browser (replace the address with your inverter's):

```
http://192.168.1.50/solar_api/v1/GetPowerFlowRealtimeData.fcgi
```

You should see a block of text containing values like `P_PV` and `P_Load`.

## Installation

1. Download the plugin file `xyz.burtscher.homematic.plugin.fronius-<version>.tar.gz` from the [releases page](https://github.com/mburtscher/HomematicIP.Net/releases) (releases named `fronius-v<version>`).
2. Open **HCUweb** and install the file as a plugin on your Home Control Unit (see the HCU manual for installing plugins from a file). Do not unpack the archive.
3. After installation, the plugin appears in HCUweb with the status **Configuration required**.

## Configuration

1. Open the plugin's page in HCUweb and open its configuration.
2. Enter the **IP address of the inverter** (for example `192.168.1.50`) in the group *Solar API*. Only IPv4 addresses in the form `a.b.c.d` are accepted.
3. Save.

When you save, the plugin checks that the inverter answers on that address. If everything is fine, the plugin switches to **Ready** and stores the address. Otherwise the configuration is rejected with a message explaining why, and nothing changes. The address is kept across updates and restarts of the plugin.

To use a different inverter or address later, open the configuration again and save the new address.

## Adding the devices

Once the plugin is ready, its four devices show up as available devices in your Homematic IP system (the same way new Homematic IP devices appear in the inbox). Include the ones you want, give them a room and a name, and they behave like any other Homematic IP device from then on.

## Troubleshooting

| Symptom | What to check |
|---|---|
| Saving the configuration fails with *"API endpoint not available under …"* | The address is wrong, the inverter is switched off or not in the network, or the Solar API is not enabled. Open the test address from above in a browser **from a computer in the same network**. |
| Saving fails with *"… is not a valid IPv4 address"* | Enter the address as four numbers separated by dots, without `http://` and without a port. |
| Plugin stays on *Configuration required* | No address has been saved yet. |
| Devices exist but show no values | The inverter could not be reached or does not report these values. Updates are skipped while the inverter does not answer, so the last known value stays; they resume automatically once it is reachable again. A system without a battery or meter has no values for those devices. |
| Battery level is missing | The inverter does not report a state of charge (no battery connected, or the battery is not reported by inverter 1). |

If you need to look closer, enable logging for the plugin in HCUweb and check its log.

## Good to know

- Only the **first inverter** of the system is used for the battery state of charge.
- The device names and model labels (for example *Wechselrichter*, battery model *BYD*) are fixed. You can rename the devices in Homematic IP at any time.
- The plugin only *reads* from the inverter; it never changes anything on it.
- If you used an earlier version of this plugin, your saved settings are picked up automatically.

## For developers

This plugin is built with the [`hmip-rs`](../../crates/hmip-rs) crate. How to build and package it: see the [repository README](../../README.md#building-a-plugin-image). To run it on your computer against a real HCU, set `HCU_HOST`, `HCU_TOKEN` and `FRONIUS_CONFIG` and use `cargo run -p fronius`.

## License

MIT, see [LICENSE](../../LICENSE).
