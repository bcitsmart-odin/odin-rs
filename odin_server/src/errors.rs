/*
 * Copyright © 2024, United States Government, as represented by the Administrator of 
 * the National Aeronautics and Space Administration. All rights reserved.
 *
 * The “ODIN” software is licensed under the Apache License, Version 2.0 (the "License"); 
 * you may not use this file except in compliance with the License. You may obtain a copy 
 * of the License at http://www.apache.org/licenses/LICENSE-2.0.
 *
 * Unless required by applicable law or agreed to in writing, software distributed under
 * the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND,
 * either express or implied. See the License for the specific language governing permissions
 * and limitations under the License.
 */

use thiserror::Error;

pub type Result<T> = std::result::Result<T, OdinServerError>;
 
#[derive(Error,Debug)]
pub enum OdinServerError {

    #[error("ODIN Actor error {0}")]
    OdinActorError( #[from] odin_actor::errors::OdinActorError),

    #[error("build error: {0}")]
    OdinBuildError( #[from] odin_build::OdinBuildError),

    #[error("unsupported resource: {0}")]
    UnsupportedResourceType(String),

    #[error("operation failed: {0}")]
    OpFailed( String ),
}

pub fn op_failed (msg: impl ToString)->OdinServerError {
    OdinServerError::OpFailed(msg.to_string())
}