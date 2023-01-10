//! This library provides functionality to extend the ChirpstackAPI version 3.11.1 with the ability to
//! create device profiles with a specification file and to create and execute rules.
//!
//! Its main purpose is to create specific rules for one or more devices, that leads to actions on
//! the same or different devices, when specific conditions are fulfilled. For an overview of a rule
//! see [this](#structure-of-a-rule) paragraph.
//!
//!
//! # Requirements
//! To use this library following requirements should be fulfilled:
//! - Working Chirpstack servers version 3.11.1 ([chirpstack.io](https://www.chirpstack.io/))
//! - LoRa messages have to be exchanged via Mqtt <br/> (should already be satisfied when the
//! servers are working; this library was tested with a [mosquitto broker](https://mosquitto.org/))
//! - In the Mqtt configuration file, there have to exist a user that is allowed to sent messages
//! and receive every incoming message, that contains payload data from a device.
//! - Uplink specification file, Downlink specification file and/or device profile specification file <br/>
//! (for more information about specification files see [this](#specification-files) paragraph).
//! - To be able to load and safe devices and device profiles, and to add a message to a device queue
//! a unique API key is required! It can be generated in the Chirpstack application server
//! (API keys > create).
//!
//! # Diagrams
//! In this section an overview of the [structure of a rule](#structure-of-a-rule) and an [uml diagram](#uml) of all methods is given.
//! ## Structure of a rule
//! ![Image](../../../documents/rule_structure.png)
//! ## UML
//! ![Image](../../../documents/class_diagram.png)
//!
//! # Specification files
//!
//! Through specification files, which include either the uplink specification, the downlink
//! specification or the whole device profile specification, it is possible to create rules that
//! execute a specific action after an uplink message is read.
//!
//! The reason for different types of specification files is that devices are often not able to
//! perform actions, therefore just the uplink data is necessary. Also there are devices that
//! don't measure anything, but can execute actions, thus the option for downlink only.<br/>
//! In addition it is also possible to load device profiles and devices out of the
//! Chirpstack sever. In that case the profile's specification should not be read via
//! the file, but directly from Chirpstack. Then just uplink or downlink specifications are read.
//!
//! The specification files are necessary to get the full functionality of this library.
//!
//! # Examples of the different specification files
//!
//! ## Device profile specification file
//! ```json
//!{
//!   "device_profile":
//!   {
//!     "name": "XXXX",
//!     "supports_class_b": false,
//!     "class_b_timeout": 0,
//!     "ping_slot_period": 0,
//!     "ping_slot_dr": 0,
//!     "ping_slot_freq": 0,
//!     "supports_class_c": false,
//!     "class_c_timeout": 0,
//!     "mac_version": "1.0.3",
//!     "reg_params_revision": "",
//!     "rx_delay_1": 0,
//!     "rx_dr_offset_1": 0,
//!     "rx_datarate_2": 0,
//!     "rx_freq_2": 0,
//!     "factory_preset_freqs":[
//!       "86800000"
//!     ],
//!     "max_eirp": 14,
//!     "max_duty_cycle": 0,
//!     "supports_join": true,
//!     "rf_region": "EU868",
//!     "supports_32bit_f_cnt": true,
//!     "payload_codec": "",
//!     "payload_encoder_script": "",
//!     "payload_decoder_script": "",
//!     "geoloc_buffer_ttl": 0,
//!     "geoloc_min_buffer_size": 0,
//!     "uplink_interval": 1200,
//!     "adr_algorithm_id": "0"
//!   },
//!   "uplink":
//!   {
//!     "payloads": [
//!       "current",
//!       "factor",
//!       "power",
//!       "power_sum",
//!       "state",
//!       "voltage"
//!     ]
//!   },
//!   "downlink":
//!   {
//!     "hex_pre_byte": "",
//!     "combined_work_load_count": false,
//!     "payloads":[
//!       {
//!        "command_name": "Open",
//!         "description": "To open socket",
//!         "configurable": false,
//!         "hex_code": "080100ff"
//!       },
//!       {
//!         "command_name": "Close",
//!         "description": "To close socket",
//!         "configurable": false,
//!         "hex_code": "080000ff"
//!       }
//!     ]
//!   }
//! }
//! ```
//!
//! ## Downlink specification file
//!
//! ```json
//! {
//!   "hex_pre_byte": "",
//!   "combined_work_load_count": false,
//!   "payloads":[
//!     {
//!       "command_name": "Open",
//!       "description": "To open socket",
//!       "configurable": false,
//!       "hex_code": "080100ff"
//!     },
//!     {
//!       "command_name": "Close",
//!       "description": "To close socket",
//!       "configurable": false,
//!       "hex_code": "080000ff"
//!     }
//!   ]
//! }
//! ```
//!
//! ## Uplink specification file
//! ```json
//! {
//!   "payloads": [
//!     "co2",
//!     "humidity",
//!     "light",
//!     "motion",
//!     "temperature",
//!     "vdd"
//!   ]
//! }
//! ```

/// This module is for the connection to the Chirpstack server and a MQTT broker. <br/>
/// It is necessary to establish these connections to get the full functionality of this library.
pub mod connections;

/// This module is for the management of devices and device profiles.
///
/// Note that it is necessary to establish a connection before loading data from or writing to the
/// Chirsptack server. Beware that [`DeviceProfileContainer::establish_connection`](devices::DeviceProfileContainer::establish_connection)
/// establishes another connection than [`DeviceContainer::establish_connection`](devices::DeviceContainer::establish_connection),
/// therefor these functions must be called in their respective cases.
///
/// Also note that there is a difference between a [`DeviceProfile`](devices::DeviceProfile) and
/// a [`DeviceProfileListItem`](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceProfileListItem.html),
/// which is here referred to as _chirpstack device profile_.<br/>
/// The same applies to [`Device`](devices::Device)s and [`DeviceListItem`](https://docs.rs/chirpstack_api/latest/chirpstack_api/as_pb/external/api/struct.DeviceListItem.html)s,
/// which are referred to as _chirpstack devices_.
///
/// In the following the usage of this module will be described with an example.
/// # Example
/// - [Startup](#startup)
/// - [Creating a device profile with a specification file](#creating-a-device-profile-with-a-specification-file)
/// - [Loading existing device profile](#loading-existing-device-profile)
/// - [Loading specification to an existing device profile](#loading-specification-to-an-existing-device-profile)
/// - [Loading a device](#loading-a-device)
///
/// ## Startup
/// It is necessary to create a device profile container and a device container.<br/>
/// To read from or to write to the Chrisptack sever a [`ChirpstackConnection`](connections::ChirpstackConnection) is needed.
/// For more information see the [connections](connections) module.
/// ```
/// use std::sync::{Arc, Mutex};
/// use elorapi::connections::ChirpstackConnection;
/// use elorapi::devices::{DeviceContainer, DeviceProfileContainer};
///
/// // create a DeviceContainer
/// let dev_container_arc_mutex = Arc::new(Mutex::new(DeviceContainer::new()));
/// let dev_container_arc = Arc::clone(&dev_container_arc_mutex);
/// let mut dev_container = dev_container_arc.lock().unwrap();
///
/// // creat a DeviceProfileContainer
/// let dev_prof_container_arc_mutex = Arc::new(Mutex::new(DeviceProfileContainer::new()));
/// let dev_prof_container_arc = Arc::clone(&dev_prof_container_arc_mutex);
/// let mut dev_prof_container = dev_prof_container_arc.lock().unwrap();
///
/// // create a ChirpstackConnection
/// let connection = ChirpstackConnection::new("API_Token", "server_uri");
/// ```
/// ## Creating a device profile with a specification file
/// To create a new device profile, it is possible to read all necessary specifications
/// out of a json file. For that the network server id and the organization id,
/// to which the profile should be written, is necessary. The ids can be seen in the Chirpstack application server.
///
/// For more information about the specification files go to the [specification files](../index.html#specification-files) paragraph.
/// ```
/// use elorapi::devices::DeviceProfile;
///
/// // the network server id and the organization id
/// let network_server_id = 11;
/// let organization_id = 1;
/// // path of the specification file
/// let path = "./specification_file_path/specification_file.json";
///
/// // read device profile out of specification file
/// let mut device_profile = DeviceProfile::read_specification(path, network_server_id, organization_id).unwrap();
///
/// // write the device profile to the Chirpstack server
/// device_profile.write_device_profile(connection.clone()).await.unwrap();
///
/// // add device profile to the container
/// dev_prof_container.add_device_profile(device_profile);
/// ```
/// ## Loading existing device profile
/// It is possible to load existing device profiles out of the Chirpstack server.<br/>
/// For this the following steps need to be done:
/// - __Establish a device profile container connection__<br/>
/// Note that this is another connection than the Chirpstack connection.
/// ```
/// dev_prof_container.establish_connection(connection.clone()).await.unwrap();
/// ```
/// - __Load chirpstack device profile__<br/>
/// Note that _Chirpstack device profiles_ should not be misunderstood as [`DeviceProfile`](devices::DeviceProfile).<br/>
/// It is only possible to load a specific number of device profiles from a specific organization and application.
/// Therefor the specific ids must be given (this information can be seen in the Chirpstack application).
/// ```
/// // number of items and ids
/// let limit = 10;
/// let organization_id = 1;
/// let application_id = 1;
///
/// // load the device profiles and add them to the container
/// dev_prof_container.load_chirpstack_device_profiles(limit, organization_id, application_id, connection.clone()).await.unwrap();
/// ```
/// - __Load actual device profile__<br/>
/// After getting the chirpstack device profile the actual device profile can now be loaded.
/// For this the device profile id is needed (to get this information, either print the item via
/// [`DeviceProfileContainer::print_list_items`](devices::DeviceProfileContainer::print_list_items) or get all items
/// via [`DeviceProfileContainer::get_chirpstack_device_profiles`](devices::DeviceProfileContainer::get_chirpstack_device_profiles) and then their id).
///```
/// use elorapi::devices::DeviceProfile;
///
/// // get the device profile id of a chirpstack device list item
/// let device_profile_id = dev_prof_container.get_chirpstack_device_list()[0].id;
///
/// // load the actual device profile form chirpstack
/// let new_device_profile = DeviceProfile::load_device_profile(device_profile_id, connection.clone()).await.unwrap();
///
/// // add the device profile to the container
/// dev_prof_container.add_device_profile(new_device_profile);
/// ```
/// ## Loading specification to an existing device profile
/// It is possible, for some cases even necessary,
/// to load uplink and/or downlink specification/s via a specification file.
/// It can be done directly after loading the device profile.
///```
/// use elorapi::devices::DeviceProfile;
///
/// // load device profile out from the server
/// let mut new_device_profile = DeviceProfile::load_device_profile("device_profile_id", connection.clone()).await.unwrap();
/// // read the specification file for downlink, uplink or both
/// new_device_profile.read_downlink("./specification_file_path/downlink_specification.json").unwrap();
/// new_device_profile.read_uplink("./specification_file_path/uplink_specification.json").unwrap();
///
/// // add device profile to the container
/// dev_prof_container.add_device_profile(new_device_profile);
/// ```
/// It is also possible to load the specification after [loading an existing device profile](#loading-existing-device-profile).
/// For this the index of the device profile in the container is needed.
/// ```
/// // get the index of the device profile
/// let index = dev_prof_container.get_device_profile_index_via_dev_prof_id("device_profile_id").unwrap();
/// // now read the downlink or uplink specification file
/// dev_prof_container.get_device_profiles()[index].read_uplink("./specification_file_path/uplink_specification_file.json").unwrap();
///```
/// ## Loading a device
/// It is possible to load existing devices from the Chirpstack server.<br/>
/// For this the following steps need to be done:
/// - __Establish a device container connection__
///```
/// // establish a connection to the Chripstack server
/// dev_container.establish_connection(connection.clone()).await.unwrap();
///```
/// - __Loading the actual device__<br/>
/// It is only possible to load a specific number of items form a specific application.
/// After loading the chirpstack devices to the container, get a device list item out of it and load the device.
///```
/// use elorapi::devices::Device;
///
/// // load a limited list of devices in a specific application
/// let limit = 10;
/// let application_id = 1;
/// dev_container.load_chirpstack_device_list(limit, application_id, connection.clone()).await.unwrap();
///
/// // get a device list item from the container
/// let device_list_item = dev_container.get_chirpstack_device_list().get(0).unwrap();
///
/// // load the device via its dev_eui
/// let device = Device::load_device(&device_list_item.dev_eui, connection.clone()).await.unwrap();
/// ```
pub mod devices;

/// This module is for the management of rules and their execution.
///
/// Note that a rule, when started, will always be executed when possible.
/// That means everytime the respective device sends a message and it contains the selected data,
/// the action will be executed. The only exception is when a time condition was set.
///
/// `RuleGenerator` contains functions to start a command line program to generate rules.
///
/// In the following the usage of this module will be described with examples.
/// # Examples
/// - [Startup](#startup)
/// - [Creating a rule depending on a device](#creating-a-rule-depending-on-a-device)
/// - [Creating conditions depending on time](#creating-conditions-depending-on-time)
/// - [Start of a rule](#start-of-a-rule)
/// ## Startup
/// To use this module it is necessary to create a rule container.
///```
/// use std::sync::{Arc, Mutex};
/// use elorapi::rules::RuleContainer;
///
/// // create a rule container
/// let rule_container_arc_mutex = Arc::new(Mutex::new(RuleContainer::new()));
/// let rule_container_arc = Arc::clone(&rule_container_arc_mutex);
/// let rule_container = rule_container_arc.lock().unwrap();
/// ```
/// ## Creating a rule depending on a device
/// There a four steps to follow to create a rule:
///  - __Create one or more condition/s__<br/>
/// Here you need to get device/s out of the device container
/// (for more information on this see the [devices] module; here the assumption is made that the device has already been loaded out of the container),<br/>
/// the index of the data you want to compare of the uplink message
/// (this can either be found in the json specification file of the respective device profile,
/// in the uplink message of the device in the chirpstack application sever or
/// by using the function [`DeviceProfile::print_uplink`](devices::DeviceProfile::print_uplink)),<br/>
/// the comparison operator and the value to compare to.
///```
/// use elorapi::rules::{Condition, DeviceCondition, RefValue};
///
/// // create first condition
/// let index = 2;
/// let comparison_operator = "<=".to_string();
/// let comparison_value = RefValue::IntNumber(1);
/// let condition_one = Condition::Device(DeviceCondition::new(device.clone(), index, comparison_operator, comparison_value));
///
/// // create second condition
/// let condition_two = Condition::Device(DeviceCondition::new(device, 4, "==".to_string(), RefValue::String("open".to_string())));
/// ```
///  - __Create an action__<br/>
/// To create an action a device is needed, where the action should be executed on,<br/>
/// the indices of the [`DownlinkPayload`](devices::DownlinkPayload)s, which should be executed
/// (this information can be found in the specification file or by using the function
/// [`DeviceProfile::print_downlink`](devices::DeviceProfile::print_downlink)),<br/>
/// the message encoded in hexadecimal, which should be sent,<br/>
/// and the port to which the message should be sent to.
///```
/// use elorapi::rules::Action;
///
/// let indices = vec![0];
/// let message = "message_in_hex".to_string();
/// let f_port = 55;
/// let action_one = Action::new(device.clone(), indices, message, f_port);
/// ```
///  - __Create the actual rule__<br/>
/// Here vectors for all conditions, boolean operators and actions are needed.
///```
/// use elorapi::rules::Rule;
///
/// let conditions = vec![condition_one, condition_two];
/// let boolean_operators = vec!["&".to_string()];
/// let actions = vec![action_one];
/// let rule = Rule::new(conditons, boolean_operators, actions);
/// ```
///  - __Add rule to the container__
/// ```
/// rule_container.add_rule(rule);
/// ```
/// ## Creating conditions depending on time
/// In this example a rule is created, which should be executed from Monday to Wednesday between 8am and 3pm.
///```
/// use chrono::{NaiveTime, Weekday};
/// use elorapi::rules::{Condition, TimeCondition};
///
/// // time
/// let timespan_start = NaiveTime::from_hms(8, 0, 0);
/// let timespan_end = NaiveTime::from_hms(15, 0, 0);
///
/// // condition monday
/// let weekday = Some(Weekday::Mon);
/// let time_condition = TimeCondition::new(weekday, timespan_start, timespan_end);
/// let condition_mon = Condition::Time(time_condition);
///
/// // condition tuesday
/// let condition_tue = Condition::Time(TimeCondition::new(Some(Weekday::Tue), timespan_start, timespan_end));
///
/// // condition wednesday
/// let condition_tue = Condition::Time(TimeCondition::new(Some(Weekday::Wed), timespan_start, timespan_end));
/// ```
/// The next example shows the condition, when you want an action to be executed everyday,
/// but only in a specific timespan. Here between 11pm and 4pm.
///```
/// use chrono::NaiveTime;
/// use elorapi::rules::{Condition, TimeCondition};
///
/// let timespan_start = NaiveTime::from_hms(23, 0, 0);
/// let timespan_end = NaiveTime::from_hms(16, 0, 0);
/// let condition = Condition::Time(TimeCondition::new(None, timespan_start, timespan_end));
/// ```
/// ## Start of a rule
/// It is necessary to establish a [`Mqtt`](connections::Mqtt) connection and a [`ChirpstackConnection`](connections::ChirpstackConnection),
/// when a rule should be executed.
/// For more information on this see the [connections] module.
///
/// To start all rules you need to have the locked rule container with rules in it,<br/>
/// the established mqtt connection,<br/>
/// the locked device profile container, which contains at least the device profiles used in the rules,<br/>
/// the established container connection
/// and the established Chirpstack connection.<br/>
///
///```
/// use elorapi::connections::{ChirpstackConnection, Mqtt};
/// use elorapi::rules::RuleContainer;
///
/// // create a Chirpstack connection
/// let chirpstack_connection = ChirpstackConnection::new("APIToken", "uri");
///
/// // create a Mqtt connection
/// let mqtt = Mqtt::new("uri", "username", "password");
/// // this starts receiving the messages from the mqtt broker and publish them into a queue
/// let receiver = mqtt.start_receiving();
///
/// // establish container connection
/// rule_container.establish_connection(chirpstack_connection.clone()).await.unwrap();
///
/// // get rule container client
/// let container_client = rule_container.get_client().unwrap();
///
/// let mut handlers = Vec::new();
/// // loop to all rules in the container and start them
/// for rule in rule_container.get_rules() {
///    // this is necessary due to the loop
///    let mqtt_clone = mqtt.clone();
///    // this is necessary due to the loop
///    let chirpstack_connection_clone = chirpstack_connection.clone();
///    // this starts the rule execution of the specific rule
///    let handler = RuleContainer::start_rule_execution(rule, mqtt_clone, &device_profile_container, container_client, chirpstack_connection_clone);
///    handlers.push(handler);
/// }
///
/// // this is necessary that the associated thread is executed
/// for join_handle in handlers {
///    join_handle.join().unwrap();
/// }
/// ```
pub mod rules;