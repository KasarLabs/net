use mp_felt::Felt252Wrapper;
use mp_transactions::HandleL1MessageTransaction;
use parity_scale_codec::{Decode, Encode};
use serde::Deserialize;

use crate::alloc::format;
use crate::alloc::string::String;
use crate::alloc::vec::Vec;
use crate::offchain_worker::OffchainWorkerError;

#[derive(Deserialize, Encode, Decode, Default, Debug)]
pub struct Message {
    /// Topics of the event.
    pub topics: Vec<String>,
    /// Data of the event.
    pub data: String,
}

#[inline(always)]
pub fn get_messages_events(from_block: u64, to_block: u64) -> String {
    format!(
        "{{
            \"jsonrpc\": \"2.0\",
        \"method\": \"eth_getLogs\",
        \"params\": [
            {{
                \"fromBlock\": \"0x{:x}\",
                \"toBlock\": \"0x{:x}\",
                \"address\": \"0xc662c410C0ECf747543f5bA90660f6ABeBD9C8c4\",
                \"topics\": [
                    \"0xdb80dd488acf86d17c747445b0eabb5d57c541d3bd7b6b87af987858e5066b2b\"
                ]
            }}
        ],
        \"id\": 0
    }}",
        from_block, to_block
    )
}

impl Message {
    /// Converts a `Message` into a transaction object.
    pub fn try_into_transaction(&self) -> Result<HandleL1MessageTransaction, OffchainWorkerError> {
        // Data at least contains a nonce and at some point the fees.
        if self.data.is_empty() {
            return Err(OffchainWorkerError::EmptyData);
        }
        // L2 contract to call.
        let contract_address = match Felt252Wrapper::from_hex_be(self.topics[2].as_str()) {
            Ok(f) => f,
            Err(_) => return Err(OffchainWorkerError::ToTransactionError),
        };

        // Function of the contract to call.
        let entry_point_selector = match Felt252Wrapper::from_hex_be(self.topics[3].as_str()) {
            Ok(f) => f,
            Err(_) => return Err(OffchainWorkerError::ToTransactionError),
        };

        // Add the from address here so it's directly in the calldata.
        let char_vec = format!("{:}{:}", self.topics[1].trim_start_matches("0x"), self.data.trim_start_matches("0x"))
            .chars()
            .collect::<Vec<char>>();
        // Split the data String into values. (The event Log(a: uin256, b: uin256, c: uin256) logs a single
        // string which is the concatenation of those fields).
        let data_map = char_vec.chunks(64).map(|chunk| chunk.iter().collect::<String>());
        // L1 message nonce.
        let nonce = u64::from_str_radix(
            data_map.clone().last().ok_or(OffchainWorkerError::ToTransactionError)?.trim_start_matches("0x"),
            16,
        )
        .map_err(|_| OffchainWorkerError::ToTransactionError)?;
        let mut calldata: Vec<Felt252Wrapper> = Vec::new();
        for val in data_map.take(self.data.len() - 2) {
            calldata.push(match Felt252Wrapper::from_hex_be(val.as_str()) {
                Ok(f) => f,
                Err(_) => return Err(OffchainWorkerError::ToTransactionError),
            })
        }
        let tx = HandleL1MessageTransaction { nonce, contract_address, entry_point_selector, calldata };

        Ok(tx)
    }
}
