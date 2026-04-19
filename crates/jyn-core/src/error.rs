// Copyright (c) 2026 Joydev GmbH (joydev.com)
// SPDX-License-Identifier: MIT

#[derive(Debug, thiserror::Error)]
pub enum JynError {
    #[error(transparent)]
    Joy(#[from] joy_core::error::JoyError),

    #[error("{0}")]
    Other(String),
}
