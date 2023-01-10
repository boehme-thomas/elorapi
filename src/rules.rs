use std::borrow::{BorrowMut};
use chirpstack_api::as_pb::external::api::{Device as ChirpstackDevice, device_queue_service_client::DeviceQueueServiceClient, DeviceQueueItem, EnqueueDeviceQueueItemRequest};
use crate::{connections::ChirpstackConnection, devices::{Device, DeviceContainer, DeviceProfile, DeviceProfileContainer}};
use std::{io, thread};
use std::io::{BufRead, ErrorKind};
use std::ops::{BitAnd, BitOr, BitXor};
use std::str::FromStr;
use std::string::String;
use std::sync::{Arc, Mutex};
use std::thread::{sleep};
use std::time::Duration;
use paho_mqtt::{Message};
use regex::Regex;
use serde_json::Value;
use tokio::{runtime::Handle, sync::watch::Receiver};
use tonic::{metadata::MetadataValue, Request, transport::{Channel, Error}};
use chrono::{Weekday, offset::Local, Datelike, NaiveTime};


/**
    To generate a [`Rule`] via a command line application.
 */
pub struct RuleGenerator {
}

impl RuleGenerator {

    /// Deciding whether a device or a Date shoule be used as condition via cmd.
    fn decision() -> Result<bool, ()> {
        let mut stdin = io::stdin().lock();
        let mut buffer = String::with_capacity(2048);
        println!("Should a device or a date and time be used for the condition?");
        println!("\"device\" for device or \"date\" for time and or date; \"brake\" ofr ending rule generation.");
        stdin.read_line(&mut buffer).expect("");
        match buffer.as_str() {
            "device\n" => Ok(true),
            "date\n" => Ok(false),
            _ => return Err(())
        }
    }

    /// Selecting a date, time or both via cmd
    fn select_date_time() -> Result<TimeCondition, ()> {
        let mut stdin = io::stdin().lock();
        let mut buffer = String::with_capacity(2048);
        println!("\n...............................Selection of date and or time................................");
        println!("Select the weekday, e.g. \"Mon\", \"Sunday\",...");
        println!("\"All\" for everyday");
        let mut weekday = None;
        'date: loop {
            stdin.read_line(&mut buffer).expect("");
            buffer = buffer.replace("\n", "");
            let res = Weekday::from_str(buffer.as_str());
            if buffer.as_str() != "All" {
                weekday = match res {
                    Ok(weekday) => Some(weekday),
                    Err(_) => {
                        buffer.clear();
                        println!("Error while parsing date!\nPleas try again:");
                        continue 'date
                    }
                };
            }
            break;
        }
        let mut timespans = Vec::new();
        let mut i = 0;
        'timespan: while i <= 1 {
            match i {
                0 => println!("Select the beginning of the timespan in which the codition should be active!"),
                1 => println!("Select the ending of the timespan in which the codition should be active!"),
                _ => {}
            }

            println!("In format of hh:mm!");

            buffer.clear();
            stdin.read_line(&mut buffer).expect("");
            buffer = buffer.replace("\n", "");
            let result = NaiveTime::parse_from_str(buffer.as_str(), "%H:%M");
            match result {
                Ok(timespan) => {
                    timespans.push(timespan);
                    i += 1;
                },
                Err(_) => {
                    println!("Error while parsing timespan!\nPleas try again:");
                    continue 'timespan
                }
            }
        }
        println!("............................................................................................");
        return Ok(TimeCondition::new(weekday, timespans[0], timespans[1]))
    }

    /// Selecting a [`Device`] via cmd.
    fn select_device(dev_container: &mut DeviceContainer) -> Result<Device, ()> {
        let mut stdin = io::stdin().lock();
        let mut buffer = String::with_capacity(2048);
        println!("\n....................................Selection of a device...................................");
        println!("Should a list of all possible devices be printed? (y/n), \"brake\" for ending rule generation");
        stdin.read_line(&mut buffer).expect("");
        match buffer.as_str() {
            "y\n" => dev_container.print_devices(),
            "n\n" => {},
            _ => return Err(()),
        }

        let device: Device;
        'device: loop {
            println!("Device index (from 0) of device that should be added:");
            buffer.clear();
            stdin.read_line(&mut buffer).expect("");
            buffer = buffer.replace("\n", "");
            let res = buffer.parse::<usize>();
            let index = match res {
                Ok(index) => index,
                Err(_) => {
                    println!("There was a Error while parsing!\nDo you want to try again? (y/n)");
                    buffer.clear();
                    stdin.read_line(&mut buffer).expect("");
                    match buffer.as_str() {
                        "y\n" => continue 'device,
                        _ => return Err(()),
                    }
                }
            };
            // get device out of device_container
            let result = dev_container.get_device(index);

            device = match result {
                Ok(device) => device,
                Err(e) => {
                    println!("{}\nDo you want to try again? (y/n)", e.to_string());
                    buffer.clear();
                    stdin.read_line(&mut buffer).expect("");
                    match buffer.as_str() {
                        "y\n" => continue 'device,
                        _ => return Err(()),
                    }
                }
            };
            println!("............................................................................................");
            return Ok(device)
        }
    }

    /// Selecting an [`Uplink`](crate::devices::Uplink) via cmd.
    fn select_uplink(device_profile: &mut DeviceProfile) -> Result<usize, ()>{
        let mut stdin = io::stdin().lock();
        let mut buffer = String::with_capacity(2048);
        println!("\n....................................Selection of measured data..............................");
        println!("Should a list of all possible measured data be printed? (y/n)\n\"brake\" for ending rule generation.");
        buffer.clear();
        stdin.read_line(&mut buffer).expect("");
        match buffer.as_str() {
            "y\n" => device_profile.print_uplink(),
            "n\n" => {},
            _ => return Err(()),
        }
        println!("Upload Payload index of Payload that should be added:");
        buffer.clear();
        stdin.read_line(&mut buffer).expect("");
        buffer = buffer.replace("\n", "");
        let dev_len = device_profile.get_uplink().unwrap().get_payloads().len();
        'payload: loop {
            let res = buffer.parse::<usize>();
            let payload_index = match res {
                Ok(index) => index,
                Err(_) => {
                    println!("There was a Error while parsing!\nDo you want to try again? (y/n)");
                    buffer.clear();
                    stdin.read_line(&mut buffer).expect("");
                    match buffer.as_str() {
                        "y\n" => {
                            buffer.clear();
                            stdin.read_line(&mut buffer).expect("");
                            buffer = buffer.replace("\n", "");
                            continue 'payload},
                        _ => return Err(()),
                    }
                }
            };

            if payload_index > dev_len {
                println!("Index is out of bounds!\nTry again!:");
                buffer.clear();
                stdin.read_line(&mut buffer).expect("");
                buffer = buffer.replace("\n", "");
                continue 'payload
            }
            println!("............................................................................................");
            return Ok(payload_index)
        }
    }

    /// Selecting a operator via cmd.
    fn select_operator(uplink: usize, device_profile: &mut DeviceProfile) -> Result<String, ()>{
        let mut stdin = io::stdin().lock();
        let mut buffer = String::with_capacity(2048);
        println!("\n..............................Selection of comparison operators.............................");
        println!("Select Comparison Operator for payload with index {} and following name:", uplink);
        println!("{:?}:", device_profile.get_uplink().unwrap().get_payloads()[uplink]);
        buffer.clear();
        stdin.read_line(&mut buffer).expect("");
        buffer = buffer.replace("\n", "");
        'operator: loop {
            let op= match buffer.as_str() {
                "<" => "<".to_string(),
                ">" => ">".to_string(),
                "<=" => "<=".to_string(),
                ">=" => ">=".to_string(),
                "==" => "==".to_string(),
                "!=" => "!=".to_string(),
                _ => {
                    println!("There was a Error while parsing!\nDo you want to try again? (y/n)");
                    buffer.clear();
                    stdin.read_line(&mut buffer).expect("");
                    match buffer.as_str() {
                        "y\n" => {
                            buffer.clear();
                            stdin.read_line(&mut buffer).expect("");
                            buffer = buffer.replace("\n", "");
                            continue 'operator
                        },
                        _ => return Err(()),
                    }
                }
            };
            println!("............................................................................................");
            return Ok(op);
        }
    }

    /// Selecting threshold via cmd.
    fn select_threshold(uplink:usize, device_profile: &mut DeviceProfile, dev_container: &mut DeviceContainer) -> Result<RefValue, ()> {
        let stdin = io::stdin();
        let mut buffer = String::with_capacity(2048);
        println!("\n...................................Selection of threshold...................................");
        let re_i32 = Regex::new(r"\d+").unwrap();
        let re_f32 = Regex::new(r"\d+[.]\d+").unwrap();
        let re_bool= Regex::new("false|true").unwrap();
        println!("Give threshold for payload with index {} and following traits:", uplink);
        println!("{:?}:", device_profile.get_uplink().unwrap().get_payloads()[uplink]);
        'threshold: loop {
            println!("Threshold:");
            println!("Type \"device\" if an uplink from another device should be use as threshold!");
            let par: RefValue;
            buffer.clear();
            stdin.read_line(&mut buffer).expect("");
            buffer = buffer.replace("\n", "");
            let buffer_str = buffer.as_str();
            if buffer_str.eq("device") {
                let res = RuleGenerator::select_device(dev_container);
                let device = match res {
                    Ok(device) => device,
                    Err(()) => return Err(())
                };
                let res_up = RuleGenerator::select_uplink(device_profile);
                let uplink_index = match res_up {
                    Ok(index) => index,
                    Err(()) => return Err(())
                };

                par = RefValue::Uplink((device, uplink_index));
            } else if re_f32.is_match(buffer_str) {
                let res = buffer.parse::<f32>();
                par = match res {
                    Ok(float) => RefValue::FloatNumber(float),
                    Err(_) => {
                        println!("There was a Error while parsing!\nDo you want to try again? (y/n)");
                        buffer.clear();
                        stdin.read_line(&mut buffer).expect("");
                        match buffer.as_str() {
                            "y\n" => {
                                continue 'threshold
                            },
                            _ => return Err(()),
                        }
                    }
                };
            } else if re_i32.is_match(buffer_str) {
                let res = buffer.parse::<i32>();
                par = match res {
                    Ok(int) => RefValue::IntNumber(int),
                    Err(_) => {
                        println!("There was a Error while parsing!\nDo you want to try again? (y/n)");
                        buffer.clear();
                        stdin.read_line(&mut buffer).expect("");
                        match buffer.as_str() {
                            "y\n" => {
                                continue 'threshold
                            },
                            _ => return Err(()),
                        }
                    }
                };
            } else if re_bool.is_match(buffer_str) {
                let res = buffer.parse::<bool>();
                par = match res {
                    Ok(bool) => RefValue::Bool(bool),
                    Err(_) => {
                        println!("There was a Error while parsing!\nDo you want to try again? (y/n)");
                        buffer.clear();
                        stdin.read_line(&mut buffer).expect("");
                        match buffer.as_str() {
                            "y\n" => {
                                continue 'threshold
                            },
                            _ => return Err(()),
                        }
                    }
                };
            } else {
                par = RefValue::String(buffer.clone());
            }
            println!("............................................................................................");
            return Ok(par);
        }
    }

    /// Selecting boolean operator via cmd.
    fn select_bool_op() -> Result<String, ()> {
        let mut stdin = io::stdin().lock();
        let mut buffer = String::with_capacity(2048);
        println!("\n...............................Selection of a boolean operator..............................");
        println!("Boolean operator, on which the last two condition should be compared (&, |, ^):");
        stdin.read_line(&mut buffer).expect("");
        buffer = buffer.replace("\n", "");
        'bool: loop {
            let bool_op= match buffer.as_str() {
                "&" => "&".to_string(),
                "|" => "|".to_string(),
                "^" => "^".to_string(),
                _ => {
                    println!("There was a Error while parsing!\nDo you want to try again? (y/n)");
                    buffer.clear();
                    stdin.read_line(&mut buffer).expect("");
                    match buffer.as_str() {
                        "y\n" => {
                            buffer.clear();
                            stdin.read_line(&mut buffer).expect("");
                            buffer = buffer.replace("\n", "");
                            continue 'bool
                        },
                        _ => return Err(()),
                    }
                }
            };
            println!("............................................................................................");
            return Ok(bool_op);
        }
    }

    /// Selecting [`DownlinkPayload`](crate::devices::DownlinkPayload)s via cmd.
    fn select_downlink(device_profile: &DeviceProfile) -> Result<Vec<usize>,()> {
        let mut stdin = io::stdin().lock();
        let mut buffer = String::with_capacity(2048);
        println!("\n....................................Selection of downlink data..............................");
        println!("Should a list of all possible downlink payloads be printed? (y/n)");
        stdin.read_line(&mut buffer).expect("");
        match buffer.as_str() {
            "y\n" => device_profile.print_downlink(),
            "n\n" => {},
            _ => return Err(()),
        }
        let mut downlink_indices: Vec<usize>= Vec::new();
        println!("Downlink index of Payload that should be added:");
        buffer.clear();
        stdin.read_line(&mut buffer).expect("");
        buffer = buffer.replace("\n", "");
        'downlink: loop {
            let res = buffer.parse::<usize>();
            let down_payload_index = match res {
                Ok(index) => index,
                Err(_) => {
                    println!("There was a Error while parsing!\nDo you want to try again? (y/n)");
                    buffer.clear();
                    stdin.read_line(&mut buffer).expect("");
                    match buffer.as_str() {
                        "y\n" => continue 'downlink,
                        _ => return Err(()),
                    }
                }
            };
            downlink_indices.push(down_payload_index);
            println!("Downlink Payload index was successfully added!");
            println!("Do you want to add another index? (y/n)");
            buffer.clear();
            stdin.read_line(&mut buffer).expect("");
            match buffer.as_str() {
                "y\n" => {
                    println!("Downlink index of Payload that should be added:");
                    buffer.clear();
                    stdin.read_line(&mut buffer).expect("");
                    buffer = buffer.replace("\n", "");
                    continue 'downlink
                },
                _ => break,
            }
        }
        println!("............................................................................................");
        return Ok(downlink_indices)
    }

    /// Selecting port via cmd.
    fn select_f_port() -> Result<u32, ()> {
        let mut stdin = io::stdin().lock();
        let mut buffer = String::with_capacity(2048);
        println!("..................................Selection of f_port.......................................");
        println!("Port to which the message should be sent:");
        loop {
            buffer.clear();
            stdin.read_line(&mut buffer).expect("");
            buffer = buffer.replace("\n", "");
            let res = buffer.parse::<u32>();
            let f_port = match res {
                Ok(index) => index,
                Err(_) => {
                    println!("There was a Error while parsing!\nDo you want to try again? (y/n)");
                    buffer.clear();
                    stdin.read_line(&mut buffer).expect("");
                    match buffer.as_str() {
                        "y\n" => continue ,
                        _ => return Err(()),
                    }
                }
            };
            println!("............................................................................................");
            return Ok(f_port);
        }
    }

    /// Changing [`Downlink`](crate::devices::Downlink) command to actual message.
    fn downlink_message(device_profile: &mut DeviceProfile, payload_indices: Vec<usize>) -> String {
        let mut downlink = device_profile.get_downlink().unwrap();
        let mut message: String = downlink.get_hex_pre_byte().clone();
        let count = downlink.get_combined_work_load_count();
        let payloads = downlink.get_payloads();
        if count {
            message += "§";
        }
        let mut stdin = io::stdin().lock();
        let mut buffer = String::with_capacity(2048);
        let regex = Regex::new(r"[\dA-Fa-f]+").unwrap();
        println!(".................................Creating of downlink message...............................");
        for i in payload_indices {
            let payload = payloads.get(i).unwrap();
            if payload.is_configurable() {
                buffer.clear();
                println!("Selected downlink data:");
                println!("{}", payload.get_command_name());
                println!("Note: Byte positions in the description are referring to the first 00 Byte form the left.");
                println!("With description: {}", payload.get_description());
                let hex_code_length = payload.get_hex_code().len();
                println!("And blank hex code: {}", payload.get_hex_code());
                println!("Your new message:");
                stdin.read_line(&mut buffer).expect("");
                buffer = buffer.replace("\n", "");
                loop {
                    if !regex.is_match(buffer.as_str()) | (buffer.len() != hex_code_length) {
                        println!("Your message don't matches the hex pattern");
                        println!("\tor is not the same length as the original message!");
                        println!("Pleas try again:");
                        buffer.clear();
                        stdin.read_line(&mut buffer).expect("");
                        buffer = buffer.replace("\n", "");
                    } else {
                        message += buffer.as_str();
                        break;
                    }
                }
            } else {
                message += payload.get_hex_code().as_str();
            }
        }
        println!("............................................................................................");
        if count {
            let length_position= message.find("§").unwrap();
            // message length - (index of first § then + 1 because index starts at 0
            // /2 because two alphanumeric letters equals one byte
            let length = (message.len() - (length_position + 1)) / 2;
            let hex = hex::encode(hex::encode((length as i8).to_be_bytes()));
            message = message.replace("§", hex.as_str());
        }
        return message;
    }

    /// Starts a an application to create a [`Rule`] via command line.
    /// After the successful creation, the rule is added to the rule container.
    pub fn start_rule_generator(dev_container: &mut DeviceContainer, dev_prof_container: &mut DeviceProfileContainer, rule_container: &mut RuleContainer) {
        let mut conditions: Vec<Condition> = Vec::new();
        let mut bool_ops: Vec<String> = Vec::new();
        let mut actions: Vec<Action> = Vec::new();

        let stdin = io::stdin();
        let mut buffer = String::with_capacity(2048);

        let mut i = 0;

        println!("---------------------------------------RULE-GENERATOR---------------------------------------");
        println!("+++++++++++++++++++++++++++++++++++Selection of condition+++++++++++++++++++++++++++++++++++");
        'condition: loop {
            // device whether device or date
            let condition_res = RuleGenerator::decision();
            let condition = match condition_res {
                Ok(cond) => cond,
                Err(()) => {
                    RuleGenerator::end_rule_generator();
                    return
                }
            };
            if condition {
                // select device
                let dev_res = RuleGenerator::select_device(dev_container);
                let dev = match dev_res {
                    Ok(device) => device,
                    Err(()) => {
                        RuleGenerator::end_rule_generator();
                        return
                    }
                };

                // get the device profile id from the device
                let id = dev.get_chirpstack_device().device.unwrap().device_profile_id;
                // get the device profile index in the device profile container
                let index = dev_prof_container.get_device_profile_index_via_dev_prof_id(&id).unwrap();
                let mut device_profile = dev_prof_container.get_device_profiles()[index].borrow_mut();

                // select uplink payloads
                let up_res = RuleGenerator::select_uplink(device_profile);
                let up = match up_res {
                    Ok(uplink) => uplink,
                    Err(()) => {
                        RuleGenerator::end_rule_generator();
                        return
                    }
                };

                // uplink copy
                let uplink_copy = up.clone();

                // select operators
                let op_res = RuleGenerator::select_operator(uplink_copy.clone(), device_profile);
                let op = match op_res {
                    Ok(operator) => operator,
                    Err(()) => {
                        RuleGenerator::end_rule_generator();
                        return
                    }
                };

                // select threshold
                let thresh_res = RuleGenerator::select_threshold(uplink_copy.clone(), &mut device_profile, dev_container);
                let thresh = match thresh_res {
                    Ok(threshold) => threshold,
                    Err(()) => {
                        RuleGenerator::end_rule_generator();
                        return
                    }
                };

                let new_con = Condition::Device(DeviceCondition::new(dev, up, op, thresh));
                conditions.push(new_con);

                i += 1;

                if i > 1 {
                    let bool_op = RuleGenerator::select_bool_op();
                    let bool = match bool_op {
                        Ok(bool) => bool,
                        Err(()) => {
                            RuleGenerator::end_rule_generator();
                            return
                        }
                    };
                    bool_ops.push(bool);
                }
            } else {
                let time_cond_res = RuleGenerator::select_date_time();
                let time_cond = match time_cond_res {
                    Ok(time) => time,
                    Err(()) => {
                        RuleGenerator::end_rule_generator();
                        return
                    }
                };
                let new_cond = Condition::Time(time_cond);
                conditions.push(new_cond);
            }
            println!("Do you want to add another condition? (y/n)");
            buffer.clear();
            stdin.read_line(&mut buffer).expect("");
            match buffer.as_str() {
                "y\n" => {
                    continue 'condition
                },
                _ => break,
            }
        }
        println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
        println!("+++++++++++++++++++++++++++++++++++++Selection of action++++++++++++++++++++++++++++++++++++");
        'action: loop {

            // select device
            let act_dev_res = RuleGenerator::select_device(dev_container);
            let act_dev = match act_dev_res {
                Ok(device) => device,
                Err(()) => {
                    RuleGenerator::end_rule_generator();
                    return
                }
            };

            // get the device profile id from the device
            let new_id = act_dev.get_chirpstack_device().device.unwrap().device_profile_id;
            // get the device profile index in the device profile container
            let new_index = dev_prof_container.get_device_profile_index_via_dev_prof_id(&new_id).unwrap();
            let action_device_profile = dev_prof_container.get_device_profiles()[new_index].borrow_mut();


            // select downlink
            let down_res = RuleGenerator::select_downlink(action_device_profile);
            let down = match down_res {
                Ok(downlink) => downlink,
                Err(()) => {
                    RuleGenerator::end_rule_generator();
                    return
                }
            };

            //change downlink message
            let hex_message = RuleGenerator::downlink_message(action_device_profile, down.clone());
            println!("message: {}", hex_message);

            // select f_port
            let f_port_res = RuleGenerator::select_f_port();
            let f_port = match f_port_res {
                Ok(f_port) => f_port,
                Err(()) => {
                    RuleGenerator::end_rule_generator();
                    return
                }
            };

            let action = Action::new(act_dev, down, hex_message, f_port);
            actions.push(action);

            println!("Do you want to add another action? (y/n)");
            buffer.clear();
            stdin.read_line(&mut buffer).expect("");
            match buffer.as_str() {
                "y\n" => {
                    continue 'action
                },
                _ => break,
            }
        }
        println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
        let rule:Arc<Mutex<Rule>> = Rule::new(conditions, bool_ops, actions);
        rule_container.add_rule(rule);

        RuleGenerator::end_rule_generator();
    }

    /// Declares end of RuleGenerator on cmd.
    fn end_rule_generator() {
        println!("------------------------------------END OF RULE GENERATOR-----------------------------------");
    }
}

/// Enum for possible threshold values.
pub enum RefValue {
    IntNumber(i32),
    String(String),
    FloatNumber(f32),
    Bool(bool),
    Uplink((Device, usize)),
}

/**
    To manage [`Rule`]s.
 */
pub struct RuleContainer {
    /// All Rules.
    rules: Vec<Arc<Mutex<Rule>>>,
    /// [Client](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/device_queue_service_client/struct.DeviceQueueServiceClient.html)
    /// to Chirpstack Server to manage a device queue.
    client: Option<DeviceQueueServiceClient<Channel>>,
}

impl RuleContainer {

    /// Creates new rule container.
    pub fn new() -> Self {
        return RuleContainer {
            rules: Vec::new(),
            client: None,
        }
    }

    /// Adds `Rule`.
    pub fn add_rule(&mut self, rule:Arc<Mutex<Rule>>) {
        self.rules.push(rule);
    }

    /// Gets all `Rule`s.
    pub fn get_rules(&mut self) -> &Vec<Arc<Mutex<Rule>>> {
        return &self.rules
    }

    /// Gets client.
    pub fn get_client(&mut self) -> Option<DeviceQueueServiceClient<Channel>> {
        return self.client.clone();
    }

    /// Starts execution of a given `Rule`.<br/>
    /// For an example see this [link](./index.html#start-of-a-rule).
    pub fn start_rule_execution(arc_rule: &Arc<Mutex<Rule>>, mut receiver: Receiver<Message>, dev_profile_container: &Arc<Mutex<DeviceProfileContainer>>, client: DeviceQueueServiceClient<Channel>, connection: ChirpstackConnection) ->  thread::JoinHandle<()> {
        println!("Start rule execution...");
        let safe = Arc::clone(&arc_rule);
        let mut rule = safe.lock().unwrap();
        rule.running = true;
        let mut topics = Vec::new();
        let mut qos = Vec::new();

        for i in &rule.conditions {
            if let Condition::Device(device_condition) = i {
                // get device and the respective topic, then add topic to vector
                let device: ChirpstackDevice = device_condition.device.get_chirpstack_device().device.unwrap();
                let dev_eui = device.dev_eui;
                let application_id = device.application_id;
                let topic = "application/".to_owned() + &application_id.to_string() + "/device/" + &dev_eui + "/event/up";
                if !topics.contains(&topic) {
                    topics.push(topic);
                    qos.push(0);
                }

                // if threshold is a device, then add the topic of this device also to the vector
                if let RefValue::Uplink(de) = &device_condition.threshold {
                    let dev = de.0.get_chirpstack_device().device.unwrap();
                    let de_eui = dev.dev_eui;
                    let de_application_id = dev.application_id;
                    let de_topic = "application/".to_owned() + &de_application_id.to_string() + "/device/" + &de_eui + "/event/up";
                    if !topics.contains(&de_topic) {
                        topics.push(de_topic);
                        qos.push(0);
                    }
                }
            }
        }

        let mut bool_ops:Vec<fn(bool, bool)->bool> = Vec::new();
        let mut messages = Vec::new();
        let dev_profile_container = Arc::clone(&dev_profile_container);
        let safe = Arc::clone(&arc_rule);
        let handle = Handle::current();
        let handler = thread::spawn(move || {
            println!("Waiting for rule...");
            let mut rule = safe.lock().unwrap();
            'control: while rule.running {
                /*
                 *  The vector 'messages' should contain as less messages as possible,
                 *  therefore if two or more conditions share the same mqtt topic,
                 *  only one message is added to the vector.
                 *  Also, only the newest message should be contained.
                 *
                 *  If two or more messages of the same topic
                 *  are received before a message of another one,
                 *  the old one will be replaced with the new one.
                 */
                println!("Waiting for messages to arrive...");
                // there are as many messages required, as there are topics
                while messages.len() < topics.len() {
                    // check if there are new messages in the channel
                    let res_seen = receiver.has_changed();
                    let seen = match res_seen {
                        Ok(bool) => bool,
                        Err(_) => {
                            println!("Error occurred!");
                            continue
                        },
                    };
                    // if there are no new messages sleep for 5 secs
                    if !seen {
                        sleep(Duration::from_secs(5));
                        continue
                    }
                    // copy the message out of the channel
                    let message = receiver.borrow_and_update().clone();
                    // if there is no data sleep for 5 secs
                    let pat = "\"data\":null";
                    let res = message.payload_str().find(pat);
                    if let Some(_) = res {
                        sleep(Duration::from_secs(5));
                        continue
                    }
                    // if the message from the channel is not for one of the topics than sleep for 5 secs
                    if !topics.contains(&message.topic().to_string().clone()) {
                        sleep(Duration::from_secs(5));
                        continue
                    }
                    // if vector 'messages' is empty, then add message to vector
                    if messages.len() == 0 {
                        messages.push(message)

                        // if vector 'messages' is not empty then check if messages contains a message
                        // from the same topic as the newest message
                        // if so, delete the old message and add new one
                    } else {
                        let mut existing: bool = false;
                        for k in 0..messages.len() {
                            let result = messages.get(k).unwrap();
                            if result.topic() == message.clone().topic() {
                                messages.remove(k);
                                messages.insert(k, message.clone());
                                existing = true;
                            }
                        }
                        if !existing {
                            messages.push(message);
                        }
                    }
                }

                let mut bool_vec: Vec<bool> = Vec::new();
                {
                    println!("Waiting for container...");
                    let mut dev_prof_container = dev_profile_container.lock().unwrap();

                    println!("Begin to check rule conditions...");
                    for i in &rule.conditions {
                        if let Condition::Device(device_condition) = i {
                            // get the id of the device, the index of the device in chirpsatck
                            // and the name of the filed in the uplink message
                            let id = device_condition.device.get_chirpstack_device().device.unwrap().device_profile_id;
                            let index = dev_prof_container.get_device_profile_index_via_dev_prof_id(&id).unwrap();
                            let uplink_name = dev_prof_container.get_device_profiles()[index].get_uplink().unwrap().get_payloads()[device_condition.measure_data].clone();

                            // get the payload out of the message, when possible
                            let message = RuleContainer::message(&messages, device_condition.device.clone());
                            let result = RuleContainer::extract_data(message);
                            let payload = match result {
                                Ok(value) => value,
                                Err(e) => {
                                    let string = "Data could not be extracted: ".to_owned() + &e.to_string();
                                    println!("{}", string);
                                    messages.clear();
                                    bool_ops.clear();
                                    continue 'control;
                                },
                            };

                            if let RefValue::IntNumber(threshold) = device_condition.threshold {
                                // parse operator to the respective function
                                let op_res = threshold.get_operator(device_condition.operator.clone());
                                let op = match op_res {
                                    Ok(op) => op,
                                    Err(()) => {
                                        println!("Error occurred: Operator could not be parsed!\nStopping rule!");
                                        rule.running = false;
                                        continue 'control
                                    }
                                };

                                // parse data from message to i32
                                let measured_data = match payload[uplink_name].as_i64() {
                                    Some(val) => val as i32,
                                    None => {
                                        println!("Error occurred while reading message.");
                                        messages.clear();
                                        bool_ops.clear();
                                        continue 'control
                                    }
                                };
                                let bool_res = op(&measured_data, &threshold);
                                bool_vec.push(bool_res);
                            } else if let RefValue::FloatNumber(threshold) = device_condition.threshold {
                                let op_res = threshold.get_operator(device_condition.operator.clone());
                                let op = match op_res {
                                    Ok(op) => op,
                                    Err(()) => {
                                        println!("Error occurred: Operator could not be parsed!\nStopping rule!");
                                        rule.running = false;
                                        continue 'control
                                    }
                                };

                                let measured_data = match payload[uplink_name].as_f64() {
                                    Some(val) => val as f32,
                                    None => {
                                        println!("Error occurred while reading message.");
                                        messages.clear();
                                        bool_ops.clear();
                                        continue 'control
                                    }
                                };
                                let bool_res = op(&measured_data, &threshold);
                                bool_vec.push(bool_res);
                            } else if let RefValue::Bool(threshold) = device_condition.threshold {
                                let op_res = threshold.get_operator(device_condition.operator.clone());
                                let op = match op_res {
                                    Ok(op) => op,
                                    Err(()) => {
                                        println!("Error occurred: Operator could not be parsed!\nStopping rule!");
                                        rule.running = false;
                                        continue 'control
                                    }
                                };

                                let measured_data = match payload[uplink_name].as_bool() {
                                    Some(val) => val,
                                    None => {
                                        println!("Error occurred while reading message.");
                                        messages.clear();
                                        bool_ops.clear();
                                        continue 'control
                                    }
                                };
                                let bool_res = op(&measured_data, &threshold);
                                bool_vec.push(bool_res);
                            } else if let RefValue::String(threshold) = &device_condition.threshold {
                                let op_res = threshold.get_operator(device_condition.operator.clone());
                                let op = match op_res {
                                    Ok(op) => op,
                                    Err(()) => {
                                        println!("Error occurred: Operator could not be parsed!\nStopping rule!");
                                        rule.running = false;
                                        continue 'control
                                    }
                                };

                                let measured_data = match payload[uplink_name].as_str() {
                                    Some(val) => val.to_string(),
                                    None => {
                                        println!("Error occurred while reading message.");
                                        messages.clear();
                                        bool_ops.clear();
                                        continue 'control
                                    }
                                };
                                let bool_res = op(&measured_data, &threshold);
                                bool_vec.push(bool_res);
                            } else if let RefValue::Uplink(threshold) = &device_condition.threshold {
                                let val = payload[uplink_name.clone()].clone();

                                let device = &threshold.0;
                                let index_second_dev = threshold.1;
                                let message = RuleContainer::message(&messages, device.clone());
                                let result = RuleContainer::extract_data(message);
                                let second_payload = match result {
                                    Ok(value) => value,
                                    Err(e) => {
                                        let string = "Data could not be extracted: ".to_owned() + &e.to_string();
                                        println!("{}", string);
                                        messages.clear();
                                        bool_ops.clear();
                                        continue 'control;
                                    },
                                };

                                let id_2 = device_condition.device.get_chirpstack_device().device.unwrap().device_profile_id;
                                let index_2 = dev_prof_container.get_device_profile_index_via_dev_prof_id(&id_2).unwrap();
                                let uplink_name_2 = dev_prof_container.get_device_profiles()[index_2].get_uplink().unwrap().get_payloads()[index_second_dev].clone();

                                if val.as_bool().is_some() {
                                    let first_measure_data = match payload[uplink_name].as_bool() {
                                        Some(val) => val,
                                        None => {
                                            println!("Error occurred while reading message.");
                                            messages.clear();
                                            bool_ops.clear();
                                            continue 'control
                                        }
                                    };
                                    let op_res = first_measure_data.get_operator(device_condition.operator.clone());
                                    let op = match op_res {
                                        Ok(op) => op,
                                        Err(()) => {
                                            println!("Error occurred: Operator could not be parsed!\nStopping rule");
                                            rule.running = false;
                                            continue 'control
                                        },
                                    };
                                    let second_measured_data = match second_payload[uplink_name_2].as_bool() {
                                        Some(val) => val,
                                        None => {
                                            println!("Error occurred while reading message.");
                                            messages.clear();
                                            bool_ops.clear();
                                            continue 'control
                                        }
                                    };
                                    let bool_res = op(&first_measure_data, &second_measured_data);
                                    bool_vec.push(bool_res);
                                } else if val.as_f64().is_some() {
                                    let first_measure_data = match payload[uplink_name].as_f64() {
                                        Some(val) => val as f32,
                                        None => {
                                            println!("Error occurred while reading message.");
                                            messages.clear();
                                            bool_ops.clear();
                                            continue 'control
                                        }
                                    };
                                    let op_res = first_measure_data.get_operator(device_condition.operator.clone());
                                    let op = match op_res {
                                        Ok(op) => op,
                                        Err(()) => {
                                            println!("Error occurred: Operator could not be parsed!\nStopping rule!");
                                            rule.running = false;
                                            continue 'control
                                        },
                                    };
                                    let second_measured_data = match second_payload[uplink_name_2].as_f64() {
                                        Some(val) => val as f32,
                                        None => {
                                            println!("Error occurred while reading message.");
                                            messages.clear();
                                            bool_ops.clear();
                                            continue 'control
                                        }
                                    };

                                    let bool_res = op(&first_measure_data, &second_measured_data);
                                    bool_vec.push(bool_res);
                                } else {
                                    let first_measure_data = match payload[uplink_name].as_str() {
                                        Some(val) => val.to_string(),
                                        None => {
                                            println!("Error occurred while reading message.");
                                            messages.clear();
                                            bool_ops.clear();
                                            continue 'control
                                        }
                                    };
                                    let op_res = first_measure_data.get_operator(device_condition.operator.clone());
                                    let op = match op_res {
                                        Ok(op) => op,
                                        Err(()) => {
                                            println!("Error occurred: Operator could not be parsed!\nStopping rule!");
                                            rule.running = false;
                                            continue 'control
                                        },
                                    };
                                    let second_measured_data = match second_payload[uplink_name_2].as_str() {
                                        Some(val) => val.to_string(),
                                        None => {
                                            println!("Error occurred while reading message.");
                                            messages.clear();
                                            bool_ops.clear();
                                            continue 'control
                                        }
                                    };

                                    let bool_res = op(&first_measure_data, &second_measured_data);
                                    bool_vec.push(bool_res);
                                }
                            }
                        }
                        if let Condition::Time(time_condition) = i {
                            let mut bool_weekday = true;
                            let mut bool_time = false;
                            let today = Local::now();
                            if time_condition.weekday.is_some() {
                                if time_condition.weekday.unwrap() != today.weekday() {
                                    bool_weekday = false;
                                }
                            }

                            let time = today.time();
                            let start = time_condition.timespan[0];
                            let end = time_condition.timespan[1];

                            // check time
                            // e.g. time: 15:00; range: 12:00-15:00
                            // 12:00 < 15:00 < 16:00
                            let bool_1 = (start < time) & (time < end) & (start < end);

                            // e.g. time: 15:00; range: 23:00-16:00
                            // 23:00 > 15:00 < 16:00
                            let bool_2 = (start > time) & (time < end) & (start > end);

                            // e.g. time: 15:00; range: 14:00-2:00
                            // 14:00 < 15:00 > 2:00
                            let bool_3 = (start < time) & (time > end) & (start > end);


                            if bool_1 | bool_2 | bool_3 {
                                bool_time = true;
                            }
                            let bool_res = bool_weekday & bool_time;
                            bool_vec.push(bool_res);
                        }
                    }
                }
                println!("Checking boolean operators...");
                for j in &rule.bool_ops {
                    let bool_op: fn(bool, bool) -> bool = match j.as_str() {
                        "&" => BitAnd::bitand,
                        "|" => BitOr::bitor,
                        "^" => BitXor::bitxor,
                        _ => {
                            println!("Error occurred: Operator could not be parsed!\nStopping rule!");
                            rule.running = false;
                            continue 'control
                        }
                    };
                    bool_ops.push(bool_op);
                }

                let mut bool_result: bool = bool_vec[0];
                if bool_vec.len() >= 2 {
                    for k in 0..bool_ops.len() {
                        let res = bool_ops[k](bool_result, bool_vec[k + 1]);
                        bool_result = res;
                    }
                }

                println!("Conditions are {}", bool_result);
                if bool_result {
                    for l in &rule.actions {
                        println!("Enqueueing message...");
                        let dev_eui = l.device.get_chirpstack_device().device.unwrap().dev_eui;
                        let clone_connection = connection.clone();
                        let clone_client = client.clone();
                        let res = handle.block_on(async {RuleContainer::enqueue_message(clone_client, dev_eui, l.f_port, l.message.clone(), clone_connection).await});
                        if let Err(e) = res {
                            println!("Message could not be enqueued: {}", e.to_string().as_str());
                            messages.clear();
                            bool_ops.clear();
                            continue 'control
                        }
                    }
                }

                messages.clear();
                bool_ops.clear();
            }
            ()
        });
        return handler;
    }

    /// Extracts data out of given Mqtt message.
    fn extract_data(message: String) -> Result<Value, io::Error> {
        let pat = "\"objectJSON\":\"";
        let res = message.find(pat);
        let begin = match res {
            Some(e) => e,
            // change that to false ?
            None => return Err(io::Error::new(ErrorKind::NotFound, "Selected uplink was not found!"))
        };
        let end = message.find("\",\"tags\":").unwrap();
        let result = message.get(begin+pat.len()..end);
        let payload_text = match result {
            Some(e) => e,
            // change that to false ?
            None => return Err(io::Error::new(ErrorKind::NotFound, "Selected uplink was not found!"))
        };
        let new_payload_text = payload_text.replace("\\", "");
        let result = serde_json::from_str::<Value>(&new_payload_text.as_str());
        let payload = match result {
            Ok(e) => e,
            // change that to false ?
            Err(e) => return Err(io::Error::new(ErrorKind::NotFound, e.to_string())),
        };
        Ok(payload)
    }

    /// Gets message for a specific `Device`.
    fn message(messages: &Vec<Message>, device: Device) -> String {
        for m in messages {
            if m.topic().contains(&device.get_chirpstack_device().device.unwrap().dev_eui.to_string()) {
                return m.payload_str().to_string();
            }
        }
        return "".to_string();
    }

    /// Establishes a new connection to Chirpstack Server to manage a device queue
    /// and adds resulting client to the container.
    pub async fn establish_connection(&mut self, connection: ChirpstackConnection) -> Result<(), Error> {
        let my_client = DeviceQueueServiceClient::connect(connection.get_uri()).await?;
        self.client = Option::from(my_client);
        Ok(())
    }

    /// Enqueues a message for a specific device, on a specific port.
    async fn enqueue_message(mut client: DeviceQueueServiceClient<Channel>, dev_eui: String, f_port: u32, data: String, connection: ChirpstackConnection) -> Result<(), io::Error>{
        let mes = hex::decode(data).unwrap();
        let data_b64 = base64::encode(mes).as_bytes().to_vec();

        let device_queue_item = DeviceQueueItem {
            dev_eui,
            confirmed: false,
            f_cnt: 0,
            f_port,
            data: data_b64,
            json_object: "".to_string(),
        };
        let enqueue_device_queue_item_request = EnqueueDeviceQueueItemRequest {
            device_queue_item: Some(device_queue_item),
        };

        let mut request = Request::new(enqueue_device_queue_item_request);
        let token = connection.get_api_token().parse::<MetadataValue<_>>();
        let token:MetadataValue<_> = match token {
            Ok(t) => t,
            Err(e) => {return Err(io::Error::new(ErrorKind::InvalidData, e))}
        };
        request.metadata_mut().insert("authorization", token.clone());

        let response= client.enqueue(request).await;
        let _response = match response {
            Ok(e) => e,
            Err(status) => return Err(io::Error::new(ErrorKind::Other, status.message().to_string() + &status.code().to_string()))
        };

        Ok(())
    }

    /// Stops the execution of a rule
    pub fn stop_rule_execution(arc_rule: &Arc<Mutex<Rule>>) {
        println!("Stop rule execution...");
        let safe = Arc::clone(arc_rule);
        let mut rule = safe.lock().unwrap();
        rule.running = false;
    }

}

/**
    Enum for possible conditions of a [`Rule`].
*/
pub enum Condition {
    Device(DeviceCondition),
    Time(TimeCondition)
}

/**
    Condition for a [`Rule`] depending on a [`Device`].
 */
pub struct DeviceCondition {
    /// Device which should be used for the condition.
    device: Device,
    /// Index of the [`Uplink`](crate::devices::Uplink) payload which should be used for the condition.
    measure_data: usize,
    /// The comparison operator used in the condition.
    operator: String,
    /// The threshold to which the measured data should be compared.
    threshold: RefValue,
}

impl DeviceCondition {
    /// Creates a new condition with the [`Device`], the `data` that should be read from the message,
    /// a comparison `operator` and the `threshold` to which the measured data should be compared to.<br/>
    /// See this [link](./index.html#creating-a-rule-depending-on-a-device) for an example.
    pub fn new(device: Device, measure_data: usize, operator: String, threshold: RefValue) -> Self {
        return DeviceCondition {
            device,
            measure_data,
            operator,
            threshold
        }
    }
}

/**
    Condition for a [`Rule`] depending on time.
*/
pub struct TimeCondition {
    /// [`Weekday`](https://docs.rs/chrono/0.4.20/chrono/enum.Weekday.html) which should be used for the condition;
    /// `None` if the condition should be executed everyday.
    weekday: Option<Weekday>,
    /// Timespan in which the [`Rule`] should be executed.
    /// Format of timespan: h:m
    timespan: [NaiveTime; 2],
}

impl TimeCondition {
    /// Creates a new condition with a `weekday`, a `start time` and an `end time`.<br/>
    /// For an example see this [link](./index.html#creating-conditions-depending-on-time).
    pub fn new(weekday: Option<Weekday>, timespan_start: NaiveTime, timespan_end: NaiveTime) -> Self {
        return TimeCondition {
            weekday,
            timespan: [timespan_start, timespan_end]
        }
    }

}

/**
    Action which should be executed, when the [`Condition`]s of a [`Rule`] are true.
 */
pub struct Action {
    /// The device on which the action is executed on.
    device: Device,
    /// The indices of the [`DownlinkPayload`](crate::devices::DownlinkPayload)s which should be executed.
    payload_indices: Vec<usize>,
    /// The actual message that is sent encoded in hex.
    message: String,
    /// The port to which the message is sent to.
    f_port: u32,
}

impl Action {
    /// Creates a new action with the `device` on which the action should be executed on,
    /// the `indices` of the [`DownlinkPayload`](crate::devices::DownlinkPayload)s, which should be executed,
    /// the actual `message` that will be sent, encoded in hex,
    /// and the `port` to which the message will be sent to.<br/>
    /// For an example see this [link](./index.html#creating-a-rule-depending-on-a-device).
    pub fn new(device: Device, payload_indices: Vec<usize>, message: String, f_port: u32) -> Self {
        return Action {
            device,
            payload_indices,
            message,
            f_port
        }
    }
}

/**
    Representation of a rule.
 */
pub struct Rule {
    /// Condition that should be satisfied.
    conditions: Vec<Condition>,
    /// Boolean operators which should be used for two conditions.
    bool_ops: Vec<String>,
    /// Actions that should be executed.
    actions: Vec<Action>,
    /// Indicator if the rule is momentarily executed.
    running: bool
}

impl Rule {
    /// Creates a new rule with `conditions` that should be satisfied,
    /// `boolean operators` which should be used for two conditions and
    /// `actions` that should be executed.<br/>
    /// For an example see this [link](./index.html#creating-a-rule-depending-on-a-device).
    pub fn new(conditions: Vec<Condition>, bool_ops: Vec<String>, actions: Vec<Action>,) -> Arc<Mutex<Self>> {
        return Arc::new(Mutex::new(Rule {
            conditions,
            bool_ops,
            actions,
            running: false,
        }))
    }

    /// Checks if rule is executed.
    pub fn is_running(&self) -> bool{
        return self.running;
    }

}

/// Trait for the selection of an comparison operator for a specific type.
trait Operator<T> {

    /// Gets the comparison operator function for a specific datatype.
    fn get_operator(&self, operator: String) -> Result<fn(&T, &T)->bool, ()>;
}

impl Operator<i32> for i32 {
    fn get_operator(&self, operator: String) -> Result<fn(&i32, &i32) -> bool ,()> {
        let op = match operator.as_str() {
            "<" => <i32 as PartialOrd<i32>>::lt,
            "<=" => <i32 as PartialOrd<i32>>::le,
            ">" => <i32 as PartialOrd<i32>>::gt,
            ">=" => <i32 as PartialOrd<i32>>::ge,
            "==" => <i32 as PartialEq<i32>>::eq,
            "!=" => <i32 as PartialEq<i32>>::ne,
            _ => return Err(())
        };
        return Ok(op);
    }
}

impl Operator<f32> for f32 {
    fn get_operator(&self, operator: String) -> Result<fn(&f32, &f32) -> bool,()> {
        let op = match operator.as_str() {
            "<" => <f32 as PartialOrd<f32>>::lt,
            "<=" => <f32 as PartialOrd<f32>>::le,
            ">" => <f32 as PartialOrd<f32>>::gt,
            ">=" => <f32 as PartialOrd<f32>>::ge,
            "==" => <f32 as PartialEq<f32>>::eq,
            "!=" => <f32 as PartialEq<f32>>::ne,
            _ => return Err(())
        };
        return Ok(op);
    }
}

impl Operator<bool> for bool {
    fn get_operator(&self, operator: String) -> Result<fn(&bool, &bool) -> bool ,()> {
        let op = match operator.as_str() {
            "<" => <bool as PartialOrd<bool>>::lt,
            "<=" => <bool as PartialOrd<bool>>::le,
            ">" => <bool as PartialOrd<bool>>::gt,
            ">=" => <bool as PartialOrd<bool>>::ge,
            "==" => <bool as PartialEq<bool>>::eq,
            "!=" => <bool as PartialEq<bool>>::ne,
            _ => return Err(())
        };
        return Ok(op);
    }
}

impl Operator<String> for String {
    fn get_operator(&self, operator: String) -> Result<fn(&String, &String) -> bool ,()> {
        let op = match operator.as_str() {
            "<" => <String as PartialOrd<String>>::lt,
            "<=" => <String as PartialOrd<String>>::le,
            ">" => <String as PartialOrd<String>>::gt,
            ">=" => <String as PartialOrd<String>>::ge,
            "==" => <String as PartialEq<String>>::eq,
            "!=" => <String as PartialEq<String>>::ne,
            _ => return Err(())
        };
        return Ok(op);
    }
}