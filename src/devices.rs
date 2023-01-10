use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::io;
use std::io::{ErrorKind};
use prost_types::Duration;
use serde_derive::Deserialize;
use chirpstack_api::as_pb::external::api::{device_profile_service_client::DeviceProfileServiceClient, device_service_client::DeviceServiceClient};
use chirpstack_api::as_pb::external::api::{DeviceProfileListItem, ListDeviceProfileRequest, GetDeviceProfileRequest, CreateDeviceProfileRequest, GetDeviceResponse, DeviceListItem, ListDeviceRequest, GetDeviceRequest};
use chirpstack_api::as_pb::external::api::DeviceProfile as ChirpstackDeviceProfile;
use serde_json::Value;
use tonic::transport::{Error};
use tonic::transport::channel::Channel;
use tonic::{Request, metadata::MetadataValue};
use crate::connections::ChirpstackConnection;


/**
    To manage [`DeviceProfile`]s and [Chirpstack device profiles](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceProfileListItem.html).
*/
#[derive(Debug)]
pub struct DeviceProfileContainer {
    /// All `DeviceProfile`s that are existing.
    device_profiles: Vec<DeviceProfile>,
    /// All [device profiles](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceProfileListItem.html), that are existing in Chirpstack.
    chirpstack_device_profiles: Vec<DeviceProfileListItem>,
    /// [Client](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/device_profile_service_client/struct.DeviceProfileServiceClient.html)
    /// to connect to the Chirpstack Server to manage device profiles.
    client: Option<DeviceProfileServiceClient<Channel>>,
}

impl DeviceProfileContainer {

    /// Creates a new device profile container.
    pub fn new() -> Self {
        return DeviceProfileContainer {
            device_profiles: Vec::new(),
            chirpstack_device_profiles: Vec::new(),
            client: None,
        }
    }

    /// Adds `DeviceProfile` to the container.
    pub fn add_device_profile(&mut self, device_profile: DeviceProfile) {
        self.device_profiles.push(device_profile);
    }

    /// Gets the index of a specific [`DeviceProfile`] via a device profile id.<br/>
    /// For an example go to the [_loading specification to an existing device profile_](./index.html#loading-specification-to-an-existing-device-profile)
    /// paragraph.
    pub fn get_device_profile_index_via_dev_prof_id(&self, dev_prof_id: &str) -> Result<usize, io::Error> {
        for i in 0..self.device_profiles.len() {
            if self.device_profiles[i].id == dev_prof_id {
                return Ok(i);
            }
        }
        return Err(io::Error::new(ErrorKind::NotFound, "DeviceProfile with specific id was not found!"));
    }

    /// Gets all `DeviceProfile`s.
    pub fn get_device_profiles(&mut self) -> &mut[DeviceProfile] {
        return self.device_profiles.borrow_mut();
    }

    /// Establishes a new connection to the Chirpstack Server to manage device profiles.
    /// Adds resulting client to the container.
    pub async fn establish_connection(&mut self, connection: ChirpstackConnection) -> Result<(), Error> {
        let my_client = DeviceProfileServiceClient::connect(connection.get_uri()).await?;
        self.client = Option::from(my_client);
        Ok(())
    }

    /// Loads a specific number of Chirpstack [`DeviceProfileListItem`](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceProfileListItem.html)s
    /// for a specific organization and application, and adds them to the container.
    /// For an example go to the [_loading existing device profile_](./index.html#loading-existing-device-profile)
    /// paragraph's second pullet point.
    pub async fn load_chirpstack_device_profiles(&mut self, limit:i64, organization_id: i64, application_id: i64, connection: ChirpstackConnection) -> Result<(), io::Error> {
        if self.client.is_none() {
            return Err(io::Error::new(ErrorKind::NotFound, "No client where found!"));
        }
        let list_request = ListDeviceProfileRequest {
            limit,
            offset: 0,
            organization_id,
            application_id
        };

        let mut request = Request::new(list_request);
        let token = connection.get_api_token().parse::<MetadataValue<_>>();
        let token:MetadataValue<_> = match token {
            Ok(t) => t,
            Err(e) => {return Err(io::Error::new(ErrorKind::InvalidData, e))}
        };
        request.metadata_mut().insert("authorization", token.clone());

        let response= self.client.as_mut().unwrap().list(request).await;
        let response = match response {
            Ok(r) => r,
            Err(e) => {return Err(io::Error::new(ErrorKind::InvalidData, e))}
        };
        for i in &response.get_ref().result {
            self.chirpstack_device_profiles.push(i.clone());
        }
        Ok(())
    }

    /// Gets all Chirpstack [`DeviceProfileListItem`](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceProfileListItem.html)s.
    /// Sometimes also referred to as _chirpstack device profile_ or the representation of these.
    pub fn get_chirpstack_device_profiles(&self) -> Vec<DeviceProfileListItem> {
        return self.chirpstack_device_profiles.clone();
    }

    /// Prints all Chirpstack [`DeviceProfileListItem`](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceProfileListItem.html)s.
    pub fn print_list_items(&self) {
        let mut j = 0;
        println!("DeviceProileListeItems:");
        for i in &self.chirpstack_device_profiles {
            println!("{}: {:?}", j, i.clone());
            j += 1;
        }
        println!("\n");
    }

    /// Prints all [`DeviceProfile`]s.
    pub fn print_device_profiles(&self) {
        println!("DeviceProfiles");
        for i in &self.device_profiles {
            println!("{:?}", i);
        }
        println!("\n");
    }
}

/**
    The representation of a device profile.
 */
#[derive(Debug)]
pub struct DeviceProfile {
    /// Id of the respective device profile in Chirpstack.
    id: String,
    /// [Device profile](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceProfile.html)
    /// on the Chirpstack Server.
    pub dev_prof: Option<ChirpstackDeviceProfile>,
    /// Possible `Uplink`.
    uplink: Option<Uplink>,
    /// Possible `Downlink`.
    downlink: Option<Downlink>,
    /// [Client](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/device_profile_service_client/struct.DeviceProfileServiceClient.html)
    /// to the Chirpstack Server to manage device profiles
    client: Option<DeviceProfileServiceClient<Channel>>,
}

impl DeviceProfile {
    /// Creates a new device profile.
    pub fn new(id: &str, uplink: Option<Uplink>, downlink: Option<Downlink>) -> DeviceProfile {
        return DeviceProfile {
            id: id.to_string(),
            dev_prof: None,
            uplink,
            downlink,
            client: None,
        };
    }

    /// Loads respective Chirpstack device profile via the device profile id
    /// and creates a new `DeviceProfile` without [`Downlink`] or [`Uplink`].
    pub async fn load_device_profile(device_profile_id: &str, connection: ChirpstackConnection) -> Result<Self, io::Error> {
        let res = DeviceProfile::establish_connection(connection.clone()).await;
        let client = match res {
            Ok(client) => client,
            Err(e) => {return Err(io::Error::new(ErrorKind::Other, e));}
        };
        let get_dev_prof = GetDeviceProfileRequest {
            id: device_profile_id.to_string()
        };
        let mut request = Request::new(get_dev_prof);
        let token = connection.get_api_token().parse::<MetadataValue<_>>();
        let token:MetadataValue<_> = match token {
            Ok(t) => t,
            Err(e) => {return Err(io::Error::new(ErrorKind::InvalidData, e))}
        };
        request.metadata_mut().insert("authorization", token.clone());
        let res = client.clone().unwrap().get(request).await;
        let dev_prof_response = match res {
            Ok(response) => response,
            Err(status) => return Err(io::Error::new(ErrorKind::Other, status.message().to_string() + &status.code().to_string()))
        };

        if !dev_prof_response.get_ref().device_profile.is_none() {
            let mut new_device_profile = DeviceProfile::new(device_profile_id, None,None);
            new_device_profile.dev_prof = dev_prof_response.get_ref().device_profile.clone();
            new_device_profile.add_client(client);
            return Ok(new_device_profile);
        }
        return Err(io::Error::new(ErrorKind::Other, "No device_profile is found!"))
    }

    /// Gets `Downlink`.
    pub fn get_downlink(&mut self) -> Option<Downlink> {
        return self.downlink.clone();
    }

    /// Reads [`Downlink`] from a json file to an already existing `DeviceProfile`.
    pub fn read_downlink(&mut self, file: &str) -> Result<(), io::Error> {
        let file_text = std::fs::read_to_string(&file)?;
        let downlink = serde_json::from_str::<Downlink>(&file_text)?;
        self.downlink = Option::from(downlink);
        Ok(())
    }

    /// Gets `Uplink`.
    pub fn get_uplink(&mut self) -> Option<Uplink> {
        return self.uplink.clone();
    }

    /// Reads [`Uplink`] form a json file to an already existing `DeviceProfile`.
    pub fn read_uplink(&mut self, file: &str) -> Result<(), io::Error> {
        let file_text = std::fs::read_to_string(&file)?;
        let uplink = serde_json::from_str::<Uplink>(&file_text)?;
        self.uplink = Option::from(uplink);
        Ok(())
    }

    /// Creates `DeviceProfile`, without `dev_prof` and `client`, out of a json file.
    pub fn read_specification(file: &str, network_server_id: i64, organization_id: i64) -> Result<DeviceProfile, io::Error> {
        if network_server_id < 1 {
            return Err(io::Error::new(ErrorKind::InvalidData, "Network server id must be greater than 0."));
        }
        if organization_id < 1 {
            return Err(io::Error::new(ErrorKind::InvalidData, "Organization id must be greater than 0."));
        }
        let file_text = std::fs::read_to_string(&file)?;
        let device_profile = serde_json::from_str::<Value>(&file_text)?;

        let mut uplink_payloads: Vec<String> = Vec::new();
        let up_payloads = device_profile["uplink"]["payloads"].as_array().ok_or(io::Error::new(ErrorKind::Other, "Error while reading uplink payloads"))?;
        let up_length = up_payloads.len();
        for j in 0..up_length {
            uplink_payloads.push(up_payloads[j].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with uplink payload"))?.to_string());
        }
        let uplink: Uplink = Uplink::new(uplink_payloads);

        let mut downlink_payloads: Vec<DownlinkPayload> = vec![];
        let payloads = device_profile["downlink"]["payloads"].as_array().ok_or(io::Error::new(ErrorKind::Other, "Error while reading downlink payloads"))?;
        let length = payloads.len();
        let mut i = 0;
        while i < length {
            let down_pay = &device_profile["downlink"]["payloads"][i];
            let command_name: String = down_pay["command_name"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with command name"))?.to_string();
            let description: String = down_pay["description"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with description"))?.to_string();
            let configurable: bool = down_pay["configurable"].as_bool().ok_or(io::Error::new(ErrorKind::Other, "No configurable field"))?;
            let hex_code: String = down_pay["hex_code"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with hex_code"))?.to_string();
            downlink_payloads.push(DownlinkPayload::new(&command_name, &description, configurable, &hex_code));
            i += 1;
        }
        let hex_pre_bytes = device_profile["downlink"]["hex_pre_byte"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with hex_pre_bytes"))?.to_string();
        let combined_work_load_count = device_profile["downlink"]["combined_work_load_count"].as_bool().ok_or(io::Error::new(ErrorKind::Other, "No combined_workload_count field"))?;
        let downlink = Downlink::new(&hex_pre_bytes, combined_work_load_count, downlink_payloads);

        let mut factory_preset_freqs_vec: Vec<u32> = vec![];
        let pres_freq_vec = device_profile["device_profile"]["factory_preset_freqs"].as_array().ok_or(io::Error::new(ErrorKind::Other, "No factory_preset_freqs field"))?;
        let pres_freq_len = pres_freq_vec.len();
        let mut j = 0;
        if pres_freq_len > 0 {
            while j < pres_freq_len {
                let payload = device_profile["device_profile"]["factory_preset_freqs"][j].as_u64().ok_or(io::Error::new(ErrorKind::Other, "No factory_preset_freqs_elemenets"))?;
                factory_preset_freqs_vec.push(payload as u32);
                j += 1;
            }
        }

        let uplink_interval: Option<Duration>;
        uplink_interval = Option::from(Duration {
            seconds: device_profile["device_profile"]["uplink_interval"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "No duration filed was found"))? as i64,
            nanos: 0,
        });

        let tags:HashMap<String, String> = HashMap::new();

        let device = ChirpstackDeviceProfile {
            id: "".to_string(),
            name: device_profile["device_profile"]["name"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with name"))?.to_string(),
            organization_id,
            network_server_id,
            supports_class_b: device_profile["device_profile"]["supports_class_b"].as_bool().ok_or(io::Error::new(ErrorKind::Other, "Problem with supports_class_b"))?,
            class_b_timeout: device_profile["device_profile"]["class_b_timeout"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem with class_b_timeout"))? as u32,
            ping_slot_period: device_profile["device_profile"]["ping_slot_period"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            ping_slot_dr: device_profile["device_profile"]["ping_slot_dr"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            ping_slot_freq: device_profile["device_profile"]["ping_slot_freq"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            supports_class_c: device_profile["device_profile"]["supports_class_c"].as_bool().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))?,
            class_c_timeout: device_profile["device_profile"]["class_c_timeout"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            mac_version: device_profile["device_profile"]["mac_version"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with mac_version"))?.to_string(),
            reg_params_revision: device_profile["device_profile"]["reg_params_revision"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with parameter_revision"))?.to_string(),
            rx_delay_1: device_profile["device_profile"]["rx_delay_1"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            rx_dr_offset_1: device_profile["device_profile"]["rx_dr_offset_1"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            rx_datarate_2: device_profile["device_profile"]["rx_datarate_2"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            rx_freq_2: device_profile["device_profile"]["rx_freq_2"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            factory_preset_freqs: factory_preset_freqs_vec,
            max_eirp: device_profile["device_profile"]["max_eirp"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            max_duty_cycle: device_profile["device_profile"]["max_duty_cycle"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            supports_join: device_profile["device_profile"]["supports_join"].as_bool().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))?,
            rf_region: device_profile["device_profile"]["rf_region"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with rf_region"))?.to_string(),
            supports_32bit_f_cnt: device_profile["device_profile"]["supports_32bit_f_cnt"].as_bool().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))?,
            payload_codec: device_profile["device_profile"]["payload_codec"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with payload_codec"))?.to_string(),
            payload_encoder_script: device_profile["device_profile"]["payload_encoder_script"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with encoder"))?.to_string(),
            payload_decoder_script: device_profile["device_profile"]["payload_decoder_script"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with decoder"))?.to_string(),
            geoloc_buffer_ttl: device_profile["device_profile"]["geoloc_buffer_ttl"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            geoloc_min_buffer_size: device_profile["device_profile"]["geoloc_min_buffer_size"].as_u64().ok_or(io::Error::new(ErrorKind::Other, "Problem reading file"))? as u32,
            tags,
            uplink_interval,
            adr_algorithm_id: device_profile["device_profile"]["adr_algorithm_id"].as_str().ok_or(io::Error::new(ErrorKind::Other, "Problem with adr_algorithm_id"))?.to_string(),
        };
        let mut final_device_profile = DeviceProfile::new(&device.id, Option::from(uplink), Option::from(downlink));
        final_device_profile.dev_prof = Option::from(device);
        Ok(final_device_profile)
    }

    /// Establishes a new connection to the Chirpstack Server to manage device profiles.
    async fn establish_connection(connection: ChirpstackConnection) -> Result<Option<DeviceProfileServiceClient<Channel>>, Error> {
        let my_client = DeviceProfileServiceClient::connect(connection.get_uri()).await?;
        Ok(Option::from(my_client))
    }

    /// Adds client.
    pub fn add_client(&mut self, client: Option<DeviceProfileServiceClient<Channel>>) {
        self.client = client;
    }

    /// Creates a new device profile in Chirpstack.
    pub async fn write_device_profile(&mut self, connection: ChirpstackConnection) -> Result<(), io::Error> {
        if self.client.is_none() {
            let res = DeviceProfile::establish_connection(connection.clone()).await;
            let client = match res {
                Ok(client) => client,
                Err(e) => {return Err(io::Error::new(ErrorKind::Other, e))}
            };
            self.client = client;
        }
        let dev_prof_req = CreateDeviceProfileRequest {
            device_profile: Option::from(self.dev_prof.clone())
        };
        if self.dev_prof.is_none() {
            return Err(io::Error::new(ErrorKind::InvalidData, "No device profile was found."));
        }
        let dev_prof = self.dev_prof.clone().unwrap();
        if dev_prof.organization_id == 0 {
            return Err(io::Error::new(ErrorKind::InvalidData, "No organization id were given."));
        }
        if dev_prof.network_server_id == 0 {
            return Err(io::Error::new(ErrorKind::InvalidData, "No network server id were given."));
        }

        let client = self.client.as_mut().unwrap();
        let mut request = Request::new(dev_prof_req);
        let token = connection.get_api_token().parse::<MetadataValue<_>>();
        let token:MetadataValue<_> = match token {
            Ok(t) => t,
            Err(e) => {return Err(io::Error::new(ErrorKind::InvalidData, e))}
        };
        request.metadata_mut().insert("authorization", token.clone());
        // println!("{:?}", client);
        // println!("{:?}", request);
        let res = client.create(request).await;
        let res = match res {
            Ok(response) => response,
            Err(status) => return Err(io::Error::new(ErrorKind::Other, status.message().to_string() + &status.code().to_string()))
        };
        let dev_ref = res.get_ref();
        self.id = dev_ref.clone().id;
        if let Some(ref mut dev_prof) = self.dev_prof {
            dev_prof.id = dev_ref.id.clone();
        }
        Ok(())
    }

    /// Prints the [`Downlink`].
    pub fn print_downlink(&self) {
        if self.downlink.is_none() {
            println!("No downlink had been added yet!");
        } else {
            println!("Donwlink for device with the id: {}", self.id);
            let down = self.downlink.as_ref().unwrap();
            println!("hex pre byte: {}", down.hex_pre_byte);
            println!("combined work load count: {}", down.combined_work_load_count.to_string());
            println!("payloads:");
            let mut j = 0;
            for i in &self.downlink.as_ref().unwrap().payloads {
                println!("{}:", j);
                println!("\tcommand name: {}", i.command_name);
                println!("\tdescription: {}", i.description);
                println!("\tconfigurable: {}", i.configurable.to_string());
                println!("\thex code: {}\n", i.hex_code);
                j += 1;
            }
        }
    }

    /// Prints the [`Uplink`].
    pub fn print_uplink(&self) {
        if self.uplink.is_none() {
            println!("No uplink had been added yet!");
        } else {
            println!("Uplink payloads for device with the id: {}", self.id);
            let mut j  = 0;
            for i in &self.uplink.as_ref().unwrap().payloads {
                println!("{}:", j);
                println!("\tMeasured Data: {}", i);
                j += 1;
            }
        }
    }
}

/**
    To store the labels of the uplink messages of a [`DeviceProfile`].
*/
#[derive(Debug, Deserialize, Clone)]
pub struct Uplink {
    /// The labels of the measured data in an uplink message.
    payloads: Vec<String>,
}

impl Uplink {
    /// Creates a new uplink.
    pub fn new(payloads: Vec<String>) -> Self {
        Uplink {
            payloads
        }
    }

    /// Adds a payload.
    pub fn add_payload(&mut self, payload_str: &str) {
        self.payloads.push(payload_str.to_string());
    }

    /// Gets all payloads.
    pub fn get_payloads(&mut self) -> &mut [String] {
        return self.payloads.borrow_mut();
    }
}

/**
    To store the structure of downlink messages of a [`DeviceProfile`].
 */
#[derive(Debug, Deserialize, Clone)]
pub struct Downlink {
    /// Header of a downlink message encoded in hex.
    hex_pre_byte: String,
    /// If the message contains a byte that represents the length of the following message.
    combined_work_load_count: bool,
    /// The representation of the messages that can be sent.
    payloads: Vec<DownlinkPayload>,
}

impl Downlink {
    /// Creates a new downlink.
    pub fn new(hex_pre_byte: &str, combined_work_load_count: bool, payloads: Vec<DownlinkPayload>) -> Self {
        Downlink {
            hex_pre_byte: hex_pre_byte.to_string(),
            combined_work_load_count,
            payloads
        }
    }

    /// Gets payload.
    pub fn get_payloads(&mut self) -> &mut [DownlinkPayload] {
        return self.payloads.borrow_mut();
    }

    /// Adds a new payload with specifications for a [`DownlinkPayload`].
    pub fn add_new_payload(&mut self, command_name: &str, description: &str, configurable: bool, hex_code: &str) {
        self.payloads.push(DownlinkPayload::new(command_name, description, configurable, hex_code));
    }
    /// Adds a new payload with an already existing [`DownlinkPayload`].
    pub fn add_payload(&mut self, downlink_payload: DownlinkPayload) {
        self.payloads.push(downlink_payload);
    }

    /// Gets `hex_pre_byte`.
    pub fn get_hex_pre_byte(&self) -> String {
        return self.hex_pre_byte.clone();
    }

    /// Gets `combined_work_load_count`.
    pub fn get_combined_work_load_count(&self) -> bool {
        return self.combined_work_load_count;
    }
}

/**
    To store the structure of downlink commands of a [`DeviceProfile`] .
 */
#[derive(Debug, Deserialize, Clone)]
pub struct DownlinkPayload {
    /// Short description of what the downlink command does.
    command_name: String,
    /// A longer description of what the command does or how it must be configured.
    description: String,
    /// If the command message can be configured.
    configurable: bool,
    /// This the actual downlink command encoded in hex.
    hex_code: String,
}

impl DownlinkPayload {
    /// Creates new downlink payload.
    pub fn new(command_name: &str, description: &str, configurable: bool, hex_code: &str) -> Self {
        DownlinkPayload {
            command_name: command_name.to_string(),
            description: description.to_string(),
            configurable,
            hex_code: hex_code.to_string(),
        }
    }
    /// Gets `configurable`.
    pub fn is_configurable(&self) -> bool {
        return self.configurable;
    }

    /// Gets `description`.
    pub fn get_description(&self) -> String {
        return self.description.clone();
    }

    /// Gets `command_name`.
    pub fn get_command_name(&self) -> String {
        return self.command_name.clone();
    }

    /// Gets `hex_code`.
    pub fn get_hex_code(&self) -> String {
        return self.hex_code.clone();
    }
}

/**
    To manage [`Device`]s.
*/
pub struct DeviceContainer {
    /// All devices.
    devices: Vec<Device>,
    /// [Devices](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceListItem.html)
    /// from the Chirpstack Server.
    chirpstack_device_list: Vec<DeviceListItem>,
    /// [Client](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/device_service_client/struct.DeviceServiceClient.html)
    /// to the Chirpstack Server to manage devices.
    client: Option<DeviceServiceClient<Channel>>
}

impl DeviceContainer {

    /// Creates new device container without a client.
    pub fn new() -> Self {
        return DeviceContainer {
            devices: Vec::new(),
            chirpstack_device_list: Vec::new(),
            client: None,
        }
    }

    /// Gets a `Device` with a specific index.
    pub fn get_device(&self, index: usize) -> Result<Device, io::Error> {
        if index > self.devices.len() {
            return Err(io::Error::new(ErrorKind::InvalidData, "Index out of bounds!"));
        }
        return Ok(self.devices[index].clone());
    }

    /// Gets the list of chripstack [devices](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceListItem.html).
    /// For an example go to the [_loading a device_](./index.html#loading-a-device) paragraph.
    pub fn get_chirpstack_device_list(&self) -> Vec<DeviceListItem> {
        return self.chirpstack_device_list.clone();
    }

    /// Adds a `Device`.
    pub fn add_device(&mut self, device: Device) {
        self.devices.push(device);
    }

    /// Establishes a new connection to the Chirpstack Server to manage devices
    /// and adds the resulting client to the container.
    pub async fn establish_connection(&mut self, connection: ChirpstackConnection) -> Result<(), Error> {
        let my_client = DeviceServiceClient::connect(connection.get_uri()).await?;
        self.client = Option::from(my_client);
        Ok(())
    }

    /// Loads a specific number of Chirpstack [`DeviceListItem`](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceListItem.html)s
    /// for a specific organization and application.<br/>
    /// For an example go to [_loading a device_](./index.html#loading-a-device) paragraph.
    pub async fn load_chirpstack_device_list(&mut self, limit:i64, application_id: i64, connection: ChirpstackConnection) -> Result<(), io::Error> {
        if self.client.is_none() {
            return Err(io::Error::new(ErrorKind::NotFound, "No client where found!"));
        }
        let list_request = ListDeviceRequest {
            limit,
            offset: 0,
            application_id,
            search: "".to_string(),
            multicast_group_id: "".to_string(),
            service_profile_id: "".to_string(),
            tags: HashMap::new(),
        };

        let mut request = Request::new(list_request);
        let token = connection.get_api_token().parse::<MetadataValue<_>>();
        let token:MetadataValue<_> = match token {
            Ok(t) => t,
            Err(e) => {return Err(io::Error::new(ErrorKind::InvalidData, e))}
        };
        request.metadata_mut().insert("authorization", token.clone());

        let response= self.client.as_mut().unwrap().list(request).await;
        let response = match response {
            Ok(r) => r,
            Err(e) => {return Err(io::Error::new(ErrorKind::InvalidData, e))}
        };
        for i in &response.get_ref().result {
            self.chirpstack_device_list.push(i.clone());
        }
        Ok(())
    }

    /// Prints all Chirpstack [devices](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.DeviceListItem.html).
    pub fn print_list_items(&self) {
        println!("DeviceListItems:");
        let mut j = 0;
        for i in &self.chirpstack_device_list {
            println!("{}:", j);
            println!("\t{:?}", i.clone());
            j += 1;
        }
    }

    /// Prints all [`Device`]s.
    pub fn print_devices(&self) {
        println!("Devices:");
        let mut j = 0;
        for i in &self.devices {
            println!("{}:", j);
            println!("\t{:?}", i.clone());
            j += 1;
        }
    }
}


/**
    The representation of a device.
 */
#[derive(Debug, Clone)]
pub struct Device {
    /// The [specifications](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.GetDeviceResponse.html) of the device on the Chirpstack Server.
    chirpstack_device: GetDeviceResponse,
    /// [Client](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/device_service_client/struct.DeviceServiceClient.html) to Chirpstack Server to manage Devices.
    client: Option<DeviceServiceClient<Channel>>
}

impl Device {
    /// Creates a new device.
    pub fn new(chirpstack_device: GetDeviceResponse) -> Self {
        return Device {
            chirpstack_device,
            client: None
        }
    }

    /// Gets the [representation](https://docs.rs/chirpstack_api/3.11.1/chirpstack_api/as_pb/external/api/struct.GetDeviceResponse.html)
    /// of the device on the Chirpstack Server.
    pub fn get_chirpstack_device(&self) -> GetDeviceResponse {
        return self.chirpstack_device.clone();
    }

    /// Establishes a new connection to Chirpstack Server to manage devices.
    async fn establish_connection(connection: ChirpstackConnection) -> Result<Option<DeviceServiceClient<Channel>>, Error> {
        let my_client = DeviceServiceClient::connect(connection.get_uri()).await?;
        Ok(Option::from(my_client))
    }

    /// Loads a device from Chirpstack with a specific dev_eui and creates a new `Device`.
    pub async fn load_device(dev_eui: &str, connection: ChirpstackConnection) -> Result<Device, io::Error> {
        let res = Device::establish_connection(connection.clone()).await;
        let client = match res {
            Ok(client) => client,
            Err(e) => {return Err(io::Error::new(ErrorKind::Other, e))}
        };
        let get_dev = GetDeviceRequest {
            dev_eui: dev_eui.to_string()
        };
        let mut request = Request::new(get_dev);
        let token = connection.get_api_token().parse::<MetadataValue<_>>();
        let token:MetadataValue<_> = match token {
            Ok(t) => t,
            Err(e) => {return Err(io::Error::new(ErrorKind::InvalidData, e))}
        };
        request.metadata_mut().insert("authorization", token.clone());
        let res = client.clone().unwrap().get(request).await;
        let dev_response = match res {
            Ok(response) => response,
            Err(status) => return Err(io::Error::new(ErrorKind::Other, status.message().to_string() + &status.code().to_string()))
        };
        let new_dev = dev_response.get_ref().device.clone();
        if new_dev.is_none() {
            return Err(io::Error::new(ErrorKind::Other, "No device_profile is found!"))
        }

        let mut new_device = Device::new(dev_response.get_ref().clone());
        new_device.add_client(client);
        return Ok(new_device);
    }

    /// Adds client.
    fn add_client(&mut self, client: Option<DeviceServiceClient<Channel>>) {
        self.client = client;
    }
}