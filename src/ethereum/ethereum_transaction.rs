/*
 * ‌
 * Hedera Rust SDK
 * ​
 * Copyright (C) 2022 - 2023 Hedera Hashgraph, LLC
 * ​
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 * ‍
 */

use hedera_proto::services;
use hedera_proto::services::smart_contract_service_client::SmartContractServiceClient;
use tonic::transport::Channel;

use crate::ledger_id::RefLedgerId;
use crate::protobuf::FromProtobuf;
use crate::transaction::{
    AnyTransactionData,
    ChunkInfo,
    ToTransactionDataProtobuf,
    TransactionData,
    TransactionExecute,
};
use crate::{
    BoxGrpcFuture,
    Error,
    FileId,
    Hbar,
    ToProtobuf,
    Transaction,
    ValidateChecksums,
};

/// Submit an Ethereum transaction.
pub type EthereumTransaction = Transaction<EthereumTransactionData>;

#[derive(Debug, Default, Clone)]
pub struct EthereumTransactionData {
    /// The raw Ethereum transaction (RLP encoded type 0, 1, and 2).
    ethereum_data: Vec<u8>,

    /// For large transactions (for example contract create) this should be used to
    /// set the FileId of an HFS file containing the call_data
    /// of the ethereum_data. The data in the ethereum_data will be re-written with
    /// the call_data element as a zero length string with the original contents in
    /// the referenced file at time of execution. The ethereum_data will need to be
    /// "rehydrated" with the call_data for signature validation to pass.
    call_data_file_id: Option<FileId>,

    /// The maximum amount that the payer of the hedera transaction
    /// is willing to pay to complete the transaction.
    max_gas_allowance_hbar: Hbar,
}

impl EthereumTransaction {
    /// Returns the raw Ethereum transaction (RLP encoded type 0, 1, and 2).
    #[must_use]
    pub fn get_ethereum_data(&self) -> &[u8] {
        &self.data().ethereum_data
    }

    /// Sets the raw Ethereum transaction (RLP encoded type 0, 1, and 2).
    pub fn ethereum_data(&mut self, data: Vec<u8>) -> &mut Self {
        self.data_mut().ethereum_data = data;
        self
    }

    /// Returns the file ID to find the raw Ethereum transaction (RLP encoded type 0, 1, and 2).
    #[must_use]
    pub fn get_call_data_file_id(&self) -> Option<FileId> {
        self.data().call_data_file_id
    }

    /// Sets a file ID to find the raw Ethereum transaction (RLP encoded type 0, 1, and 2).
    ///
    /// For large transactions (for example contract create) this should be used to
    /// set the [`FileId`] of an HFS file containing the `call_data`
    /// of the `ethereum_data`. The data in `the ethereum_data` will be re-written with
    /// the `call_data` element as a zero length string with the original contents in
    /// the referenced file at time of execution. `The ethereum_data` will need to be
    /// "rehydrated" with the `call_data` for signature validation to pass.
    pub fn call_data_file_id(&mut self, id: FileId) -> &mut Self {
        self.data_mut().call_data_file_id = Some(id);
        self
    }

    /// Returns the maximum amount that the payer of the hedera transaction
    /// is willing to pay to complete the transaction.
    #[must_use]
    pub fn get_max_gas_allowance_hbar(&self) -> Hbar {
        self.data().max_gas_allowance_hbar
    }

    /// Sets the maximum amount that the payer of the hedera transaction
    /// is willing to pay to complete the transaction.
    pub fn max_gas_allowance_hbar(&mut self, allowance: Hbar) -> &mut Self {
        self.data_mut().max_gas_allowance_hbar = allowance;
        self
    }
}

impl TransactionData for EthereumTransactionData {}

impl TransactionExecute for EthereumTransactionData {
    fn execute(
        &self,
        channel: Channel,
        request: services::Transaction,
    ) -> BoxGrpcFuture<'_, services::TransactionResponse> {
        Box::pin(async { SmartContractServiceClient::new(channel).call_ethereum(request).await })
    }
}

impl ValidateChecksums for EthereumTransactionData {
    fn validate_checksums(&self, ledger_id: &RefLedgerId) -> Result<(), Error> {
        self.call_data_file_id.validate_checksums(ledger_id)
    }
}

impl ToTransactionDataProtobuf for EthereumTransactionData {
    fn to_transaction_data_protobuf(
        &self,
        chunk_info: &ChunkInfo,
    ) -> services::transaction_body::Data {
        let _ = chunk_info.assert_single_transaction();

        let call_data = self.call_data_file_id.to_protobuf();

        services::transaction_body::Data::EthereumTransaction(services::EthereumTransactionBody {
            ethereum_data: self.ethereum_data.clone(),
            call_data,
            max_gas_allowance: self.max_gas_allowance_hbar.to_tinybars(),
        })
    }
}

impl From<EthereumTransactionData> for AnyTransactionData {
    fn from(transaction: EthereumTransactionData) -> Self {
        Self::Ethereum(transaction)
    }
}

impl FromProtobuf<services::EthereumTransactionBody> for EthereumTransactionData {
    fn from_protobuf(pb: services::EthereumTransactionBody) -> crate::Result<Self> {
        Ok(Self {
            ethereum_data: pb.ethereum_data,
            call_data_file_id: Option::from_protobuf(pb.call_data)?,
            max_gas_allowance_hbar: Hbar::from_tinybars(pb.max_gas_allowance),
        })
    }
}

#[cfg(test)]
mod tests {
    use expect_test::expect;

    use crate::transaction::test_helpers::{
        transaction_body,
        unused_private_key,
        TEST_NODE_ACCOUNT_IDS,
        TEST_TX_ID,
    };
    use crate::{
        AnyTransaction,
        EthereumTransaction,
        Hbar,
    };

    fn make_transaction() -> EthereumTransaction {
        let mut tx = EthereumTransaction::new();

        tx.node_account_ids(TEST_NODE_ACCOUNT_IDS)
            .transaction_id(TEST_TX_ID)
            .ethereum_data(vec![0xde, 0xad, 0xbe, 0xef])
            .call_data_file_id("4.5.6".parse().unwrap())
            .max_gas_allowance_hbar("3".parse().unwrap())
            .max_transaction_fee(Hbar::new(1))
            .freeze()
            .unwrap()
            .sign(unused_private_key());

        tx
    }

    #[test]
    fn serialize() {
        let tx = make_transaction();

        let tx = transaction_body(tx);

        expect![[r#"
            TransactionBody {
                transaction_id: Some(
                    TransactionId {
                        transaction_valid_start: Some(
                            Timestamp {
                                seconds: 1554158542,
                                nanos: 0,
                            },
                        ),
                        account_id: Some(
                            AccountId {
                                shard_num: 0,
                                realm_num: 0,
                                account: Some(
                                    AccountNum(
                                        5006,
                                    ),
                                ),
                            },
                        ),
                        scheduled: false,
                        nonce: 0,
                    },
                ),
                node_account_id: Some(
                    AccountId {
                        shard_num: 0,
                        realm_num: 0,
                        account: Some(
                            AccountNum(
                                5005,
                            ),
                        ),
                    },
                ),
                transaction_fee: 100000000,
                transaction_valid_duration: Some(
                    Duration {
                        seconds: 120,
                    },
                ),
                generate_record: false,
                memo: "",
                data: Some(
                    EthereumTransaction(
                        EthereumTransactionBody {
                            ethereum_data: [
                                222,
                                173,
                                190,
                                239,
                            ],
                            call_data: Some(
                                FileId {
                                    shard_num: 4,
                                    realm_num: 5,
                                    file_num: 6,
                                },
                            ),
                            max_gas_allowance: 300000000,
                        },
                    ),
                ),
            }
        "#]]
        .assert_debug_eq(&tx)
    }

    #[test]
    fn to_from_bytes() {
        let tx = make_transaction();

        let tx2 = AnyTransaction::from_bytes(&tx.to_bytes().unwrap()).unwrap();

        let tx = transaction_body(tx);

        let tx2 = transaction_body(tx2);

        assert_eq!(tx, tx2);
    }
}
