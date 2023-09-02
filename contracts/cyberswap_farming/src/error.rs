use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OverflowError(#[from] OverflowError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Not Reward or Stake token")]
    UnacceptableToken {},

    #[error("Invalid zero amount")]
    InvalidZeroAmount {},

    #[error("InvalidInput")]
    InvalidInput {},

    #[error("Not enough Reward")]
    NotEnoughReward { },

    #[error("Asset mismatch")]
    AssetMismatch {},

    #[error("Too small offer amount")]
    TooSmallOfferAmount {},

    #[error("Still in Lock period")]
    StillInLock { },

    #[error("Disabled")]
    Disabled {},
}
