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

use std::path::Path;
use tokio;
use odin_common::define_cli;
use odin_sentinel::{load_config, Alarm, AlarmMessenger, EvidenceInfo, 
    ConsoleAlarmMessenger, SmtpAlarmMessenger, SignalCmdAlarmMessenger, SlackAlarmMessenger, SentinelFile
};
use anyhow::Result;
 
define_cli! { ARGS [about="Delphire Sentinel Slack alarm test"] = 
    slack: bool       [help="enable slack messenger", long],
    smtp: bool        [help="enable smtp messenger", long],
    signal_cli: bool  [help="enable signal-cli messenger (requires signal-cli installation)", long],

    img: Option<String>    [help="optional pathname of image to attach", short, long]
}

/// test application for alarm messengers - this sends artificial alarms to the messenger types
/// specified as command line arguments (console is always enabled)
/// Note this uses the same config files from the ODIN installation as the sentinel_alarm server
#[tokio::main]
async fn main()->Result<()> {    
    let alarm = if let Some(img) = &ARGS.img {
        let pathname = Path::new(&img).to_path_buf();
        if !pathname.is_file() { panic!("image file does not exist: {img}") }
        Alarm { 
            description: "test alarm".to_string(), 
            evidence_info: vec!( 
                EvidenceInfo { 
                    description: "visual".to_string(), 
                    img: Some(SentinelFile { record_id: "image".to_string(), pathname })
                }
            ) 
        }
    } else {
        Alarm { 
            description: "test alarm".to_string(), 
            evidence_info: Vec::new() 
        }
    };

    let messengers = create_messengers()?;
    
    for m in &messengers {
        let res = m.send_alarm(&alarm).await?;
        println!("result = {res:?}");
    }

    Ok(())
}

fn create_messengers()->Result<Vec<Box<dyn AlarmMessenger>>> {
    let mut messengers: Vec<Box<dyn AlarmMessenger>> = Vec::new();

    messengers.push( Box::new(ConsoleAlarmMessenger{})); // always enabled

    if ARGS.slack {
        messengers.push( Box::new( SlackAlarmMessenger::new( load_config("slack_alarm.ron")?)))
    }
    if ARGS.smtp { 
        messengers.push( Box::new( SmtpAlarmMessenger::new( load_config("smtp")?))) 
    }
    if ARGS.signal_cli { 
        messengers.push( Box::new( SignalCmdAlarmMessenger::new( load_config("signal_cmd")?))) 
    }

    Ok(messengers)
}