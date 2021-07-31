// Copyright (C) 2019-2021 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{Ledger, TransactionLocation, COL_TRANSACTION_LOCATION};
use snarkvm_dpc::{Parameters, RecordCommitmentTree, RecordSerialNumberTree, Transaction, TransactionScheme};
use snarkvm_ledger::{errors::StorageError, BlockHeaderHash, LedgerScheme, Storage};
use snarkvm_utilities::{
    bytes::{FromBytes, ToBytes},
    has_duplicates,
    to_bytes_le,
};

impl<C: Parameters, S: Storage> Ledger<C, S> {
    /// Returns a transaction location given the transaction ID if it exists. Returns `None` otherwise.
    pub fn get_transaction_location(&self, transaction_id: &[u8]) -> Result<Option<TransactionLocation>, StorageError> {
        match self.storage.get(COL_TRANSACTION_LOCATION, transaction_id)? {
            Some(transaction_locator) => {
                let transaction_location = TransactionLocation::read_le(&transaction_locator[..])?;
                Ok(Some(transaction_location))
            }
            None => Ok(None),
        }
    }

    /// Returns a transaction given the transaction ID if it exists. Returns `None` otherwise.
    pub fn get_transaction(&self, transaction_id: &[u8]) -> Result<Option<Transaction<C>>, StorageError> {
        match self.get_transaction_location(transaction_id)? {
            Some(transaction_location) => {
                let block_transactions =
                    self.get_block_transactions(&BlockHeaderHash(transaction_location.block_hash))?;
                Ok(block_transactions.0.get(transaction_location.index as usize).cloned())
            }
            None => Ok(None),
        }
    }

    /// Returns a transaction in bytes given a transaction ID.
    pub fn get_transaction_bytes(&self, transaction_id: &[u8]) -> Result<Vec<u8>, StorageError> {
        match self.get_transaction(transaction_id)? {
            Some(transaction) => Ok(to_bytes_le![transaction]?),
            None => Err(StorageError::InvalidTransactionId(hex::encode(&transaction_id))),
        }
    }

    /// Returns true if the transaction has internal parameters that already exist in the ledger.
    pub fn transaction_conflicts<T: TransactionScheme>(&self, transaction: &T) -> bool {
        let transaction_serial_numbers = transaction.old_serial_numbers();
        let transaction_commitments = transaction.new_commitments();

        // Check if the transactions in the block have duplicate serial numbers
        if has_duplicates(transaction_serial_numbers) {
            return true;
        }

        // Check if the transactions in the block have duplicate commitments
        if has_duplicates(transaction_commitments) {
            return true;
        }

        // Check if each transaction serial number previously existed in the ledger
        for sn in transaction_serial_numbers {
            // TODO (howardwu): Remove the use of ToBytes to FromBytes.
            if self.contains_serial_number(&FromBytes::read_le(&*sn.to_bytes_le().unwrap()).unwrap()) {
                return true;
            }
        }

        // Check if each transaction commitment previously existed in the ledger
        for cm in transaction_commitments {
            // TODO (howardwu): Remove the use of ToBytes to FromBytes.
            if self.contains_commitment(&FromBytes::read_le(&*cm.to_bytes_le().unwrap()).unwrap()) {
                return true;
            }
        }

        false
    }
}
