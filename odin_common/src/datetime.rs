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

use chrono::{DateTime, TimeDelta, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc};
use serde::{Serialize,Deserialize,Serializer,Deserializer};
use std::time::{Duration, UNIX_EPOCH, SystemTime};
use std::ffi::OsStr;
use parse_duration::parse;
use crate::if_let;

pub fn epoch_millis ()->i64 {
    let now = Utc::now();
    now.timestamp_millis()
}

pub fn to_epoch_millis<Tz> (date: DateTime<Tz>)->i64 where Tz: TimeZone {
    date.timestamp_millis()
}

/// return the full hour for given DateTime (minutes, seconds and nanos all zeroed)
pub fn full_hour<Tz:TimeZone> ( dt: &DateTime<Tz>)->DateTime<Tz> {
    dt.with_minute(0).unwrap().with_second(0).unwrap().with_nanosecond(0).unwrap()
}

/// return minutes since given given DateTime<Utc> (negative if in future)
pub fn elapsed_minutes_since (dt: &DateTime<Utc>) -> i64 {
    let now = chrono::offset::Utc::now();
    (now - *dt).num_minutes()
}

pub fn duration_since (dt_later: &DateTime<Utc>, dt_earlier: &DateTime<Utc>)->Duration {
    if dt_later >= dt_earlier {
        (*dt_later - *dt_earlier).to_std().unwrap()
    } else { 
        Duration::ZERO
    }
}

pub fn is_between_inclusive (dt: &DateTime<Utc>, dt_start: &DateTime<Utc>, dt_end: &DateTime<Utc>) -> bool {
    dt >= dt_start && dt <= dt_end
}

/// get a DateTime<Utc> from a NaiveDate that is supposed to be in Utc
pub fn naive_utc_date_to_utc_datetime (nd: NaiveDate) -> DateTime<Utc> {
    let nt = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // 00:00:00 can't fail
    let ndt = NaiveDateTime::new(nd,nt);

    //DateTime::from_utc(ndt, Utc)
    DateTime::from_naive_utc_and_offset(ndt,Utc)
}

pub fn naive_local_date_to_utc_datetime (nd: NaiveDate) -> Option<DateTime<Utc>> {
    let nt = NaiveTime::from_hms_opt(0, 0, 0).unwrap(); // 00:00:00 can't fail
    let ndt = NaiveDateTime::new(nd,nt);

    // yeah - this can actually fail if the timezone changed during respective period
    Local.from_local_datetime(&ndt).single().map(|ldt| ldt.with_timezone(&Utc))
}

//--- support for serde

pub fn ser_short_rfc3339<S: Serializer> (dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error>  {
    let dfm = format!("{}", dt.format("%Y-%m-%dT%H:%M:%S%Z"));
    s.serialize_str(&dfm)
}

pub fn ser_epoch_millis<S: Serializer> (dt: &DateTime<Utc>, s: S) -> Result<S::Ok, S::Error>  {
    s.serialize_i64(dt.timestamp_millis())
}

/// NOTE if the option is None and this should not be serialized as 0 the field has to have a #[serde(skip_serializing_if="Options::is_none")] attribute
pub fn ser_epoch_millis_option<S: Serializer> (opt: &Option<DateTime<Utc>>, s: S) -> Result<S::Ok, S::Error>  {
    if let Some(dt) = opt {
        s.serialize_i64(dt.timestamp_millis())
    } else {
        s.serialize_i64(0)
    }
}

pub fn deserialize_duration <'a,D>(deserializer: D) -> Result<Duration,D::Error>
    where D: Deserializer<'a>
{
    String::deserialize(deserializer).and_then( |string| {
        parse(string.as_str())
            .map_err( |e| serde::de::Error::custom(format!("{:?}",e)))
    })
}

pub fn deserialize_optional_duration <'a,D>(deserializer: D) -> Result<Option<Duration>,D::Error> 
    where D: Deserializer<'a>
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    if let Some(s) = s {
        let d =  parse(s.as_str()).map_err( |e| serde::de::Error::custom(format!("{:?}",e)))?;
        return Ok( Some(d) )
    }

    Ok(None)
}

pub fn serialize_duration<S: Serializer> (dur: &Duration, s: S) -> Result<S::Ok, S::Error>  {
    let dfm = format!("{:?}", dur);
    s.serialize_str(&dfm)
}

pub fn serialize_optional_duration<S>(dur: &Option<Duration>, s: S) -> Result<S::Ok, S::Error>
    where S: Serializer,
{
    if let Some(ref d) = *dur {
        let dfm = format!("{:?}", dur);
        return s.serialize_str(&dfm);
    }
    s.serialize_none()
}

//--- support for structopt parsers

pub fn parse_utc_datetime_from_os_str_date (s: &OsStr) -> DateTime<Utc> {
    let nd = NaiveDate::parse_from_str(s.to_str().unwrap(), "%Y-%m-%d").unwrap();
    naive_local_date_to_utc_datetime(nd).unwrap()
}

//--- misc string format parsing

fn parse_datetime (s: &str)->Option<DateTime<Utc>> {
    match DateTime::parse_from_str(s, "%+") {
        Ok(dt) => Some(dt.to_utc()),
        Err(_) => None
    }
}

/* #region dated objects ****************************************************************************************/

/// a type bound for something we can get a date for.
/// The main purpose of this trait is to avoid having to extract DateTime lists out of already existing collections
pub trait Dated {
    fn date (&self)->DateTime<Utc>;
}

//--- some blanket impls

impl<Tz:TimeZone> Dated for DateTime<Tz> {
    fn date (&self)->DateTime<Utc> { self.to_utc() }
}

impl Dated for SystemTime {
    /// note this might panic if the SystemTime is before UNIX_EPOCH or nanos are outside of i64 (in year 2262)
    fn date (&self)->DateTime<Utc> {
        DateTime::from_timestamp_nanos( self.duration_since( UNIX_EPOCH).unwrap().as_nanos() as i64)
    }
}

/* #endregion dated objects */