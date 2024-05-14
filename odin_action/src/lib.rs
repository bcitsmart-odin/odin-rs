/*
 * Copyright (c) 2024, United States Government, as represented by the
 * Administrator of the National Aeronautics and Space Administration.
 * All rights reserved.
 *
 * The ODIN - Open Data Integration Framework is licensed under the
 * Apache License, Version 2.0 (the "License"); you may not use this file
 * except in compliance with the License. You may obtain a copy of the
 * License at http://www.apache.org/licenses/LICENSE-2.0.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
#![allow(unused_macros)]

#[doc = include_str!("../doc/odin_action.md")]

use std::{fmt::Debug, marker::PhantomData, future::{Future,ready}, 
    any::{type_name,type_name_of_val}, ops::{Deref,DerefMut},
};
pub use async_trait::async_trait;

/// return only the last part of a type path
pub fn abbrev_type_name<T>()->String {
    let full_name = type_name::<T>();

    match full_name.rfind(':') {
        None => full_name.to_string(),
        Some(idx) => unsafe { full_name.get_unchecked(idx+1..).to_string() }
    }
}

/// wrapper error type for actions.
/// We do need a dedicated error type for actions so that they can be easily mapped or coerced within
/// client modules such as odin_actor.
/// However, we also want to support arbitrary Results within the client-side defined action expression,
/// which should allow the '?' operator. Hence we have to be able to generate OdinActionErrors from anything 
/// that implements Error. 
pub struct OdinActionError {
    pub(crate) msg: String,
    pub(crate) src: String
}

impl OdinActionError {
    pub fn from<E> (e: E)->Self where E: ToString {
        OdinActionError { 
            msg: e.to_string(),
            src: std::any::type_name_of_val(&e).to_string()
        }
    }
}

impl std::fmt::Debug for OdinActionError {
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OdinActionError({}): {}", self.src, self.msg)
    }
}

impl std::fmt::Display for OdinActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "action failed: {}", self.msg)
    }
}

impl std::error::Error for OdinActionError {
}

#[inline]
pub fn map_action_err<T,E> (r: std::result::Result<T,E>)->std::result::Result<T,OdinActionError> where E: ToString {
    r.map_err( |e| OdinActionError{ msg: e.to_string(), src: type_name_of_val(&e).to_string()})
}

pub fn action_err (msg: impl ToString)->std::result::Result<(),OdinActionError> {
    Err(OdinActionError{ msg: msg.to_string(), src: "".to_string()})
}

#[inline]
pub fn action_ok()->std::result::Result<(),OdinActionError> { Ok(()) }


/* #region DataAction ************************************************************************/

/// a trait that includes a single `execute(&self,T)` method returning a future.
/// This is used as a type constraint for types that represent parameterized async actions taking
/// a single data argument.
pub trait DataAction<T>: Debug + Send {
    fn execute (&self, data: T) -> impl std::future::Future<Output = std::result::Result<(),OdinActionError>> + Send;
}

/// macro to define and instantiate ad hoc action types that clone-capture local vars and take a single
/// `execute(data)`` argument. See [module] doc for general use and expansion.
#[macro_export]
macro_rules! data_action {
    ( $( $v:ident $(. $op:ident ())? : $v_type:ty ),* => |$data:ident : $data_type:ty| $e:expr ) => {
        {
            struct SomeDataAction { $( $v: $v_type ),* }

            impl DataAction<$data_type> for SomeDataAction {
                async fn execute (&self, $data : $data_type) -> std::result::Result<(),OdinActionError> {
                    $( let $v = &self. $v;)*
                    map_action_err($e)
                }
            }
            impl std::fmt::Debug for SomeDataAction {
                fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "DataAction<{}>", stringify!($data_type))
                }
            }

            SomeDataAction{ $( $v: $v $(. $op () )? ),* }
        }
    }
}

/// an empty `DataAction<T>`. Transparent alternative for `Option<DataAction<T>>`
pub struct NoDataAction<T> where T: Send { _phantom: PhantomData<T> }

impl<T> DataAction<T> for NoDataAction<T> where T: Send {
    fn execute (&self, _data: T) -> impl Future<Output = std::result::Result<(),OdinActionError>> + Send { ready(Ok(()) )}
}
impl<T> Debug for NoDataAction<T> where T: Send {
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoDataAction<{}>", abbrev_type_name::<T>())
    }
}

pub fn no_data_action<T>()->NoDataAction<T> where T: Send { NoDataAction { _phantom: PhantomData } }

/// a [`DataAction<T>`] with an `async execute(..)` function that takes a second `bidata` parameter.
/// This can be used for actions that combine owned and passed-in data in their action bodies.
/// 
/// Note there is no corresponding `BiDynDataAction` as this normally would be a [`DynDataAction`]
/// that captures the bidata. `BiDataAction` is a way to avoid the associated runtime cost of dyn actions
/// if requester and actor both know the bidata type and the requester can pass it in through a message.
pub trait BiDataAction<T,A>: Debug + Send {
    fn execute (&self, data: T, bidata: A) -> impl std::future::Future<Output = std::result::Result<(),OdinActionError>> + Send;
}

/// macro to define and instantiate ad hoc actions taking two data arguments.
/// See [module] doc for general use and expansion.
#[macro_export]
macro_rules! bi_data_action {
    ( $( $v:ident $(. $op:ident ())? : $v_type:ty ),* => |$data:ident : $data_type:ty, $bidata:ident: $bidata_type:ty| $e:expr ) => {
        {
            struct SomeBiDataAction { $( $v: $v_type ),* }

            impl BiDataAction<$data_type,$bidata_type> for SomeBiDataAction {
                async fn execute (&self, $data : $data_type, $bidata : $anned_type) -> std::result::Result<(),OdinActionError> {
                    $( let $v = &self. $v;)*
                    map_action_err($e)
                }
            }
            impl std::fmt::Debug for SomeBiDataAction {
                fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "BiDataAction<{},{}>", stringify!($data_type),stringify!($bidata_type))
                }
            }

            SomeBiDataAction{ $( $v: $v $(. $op () )? ),* }
        }
    }
}

/// an empty [`BiDataAction<T,A>`]. Transparent alternative for `Option<DataAction<T,A>>`
pub struct NoBiDataAction<T,A> where T: Send, A: Send { _phantom1: PhantomData<T>, _phantom2: PhantomData<A> }

impl<T,A> BiDataAction<T,A> for NoBiDataAction<T,A> where T: Send, A: Send {
    fn execute (&self, _data: T, _bidata: A) -> impl std::future::Future<Output = std::result::Result<(),OdinActionError>> + Send { ready(Ok(()) )}
}
impl<T,A> Debug for NoBiDataAction<T,A> where T: Send, A: Send {
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoBiDataAction<{},{}>", abbrev_type_name::<T>(), abbrev_type_name::<A>())
    }
}
pub fn no_bi_data_action<T,A>()->NoBiDataAction<T,A>  where T: Send, A: Send { 
    NoBiDataAction { _phantom1: PhantomData, _phantom2: PhantomData } 
}

/// a sendable [`DataAction<T>`] that can be stored in a homogenous container (as respective trait objects).
/// This trait is mostly used implicitly through the corresponding [`DynDataAction<T>`] type.
/// Note: this has per-execution runtime cost as the returned future needs to be pinboxed
#[async_trait]
pub trait DynDataActionTrait<T>: Debug + Send + Sync {
    async fn execute (&self, data: T) -> std::result::Result<(),OdinActionError>;
}

/// a type alias for a boxed [`DynDataActionTrait<T>`] trait object, used to send and store respective actions. 
pub type DynDataAction<T> = Box<dyn DynDataActionTrait<T>>; 

/// macro to define and instantiate ad hoc [`DynDataAction<T>`] types.
/// See [module] doc for general use and expansion.
/// To be used where actions have to be send and/or stored in homogenous containers (as trait objects) 
#[macro_export]
macro_rules! dyn_data_action {
    ( $( $v:ident $(. $op:ident ())? : $v_type:ty ),* => |$data:ident : $data_type:ty| $e:expr ) => {
        {
            use async_trait::async_trait;

            struct SomeDynDataAction { $( $v: $v_type ),* }

            #[async_trait]
            impl odin_action::DynDataActionTrait<$data_type> for SomeDynDataAction {
                async fn execute (&self, $data : $data_type) -> std::result::Result<(),OdinActionError> {
                    $( let $v = &self. $v;)*
                    map_action_err($e)
                }
            }
            impl std::fmt::Debug for SomeDynDataAction {
                fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "DynDataAction<{}>", stringify!($data_type))
                }
            }

            Box::new(SomeDynDataAction{ $( $v: $v $(. $op () )? ),* })
        }
    }
}


/* #endregion DataAction */

/* #region DataRefAction *********************************************************************/

/// analoguous to [`DataAction<T>`] trait but taking a reference argument 
pub trait DataRefAction<T>: Debug + Send {
    fn execute (&self, data: &T) -> impl std::future::Future<Output = std::result::Result<(),OdinActionError>> + Send;
}

/// macro to define and instantiate ad hoc actions taking a single reference argument. 
/// See [module] doc for general use and expansion.
#[macro_export]
macro_rules! dataref_action {
    ( $( $v:ident $(. $op:ident ())? : $v_type:ty ),* => |$data:ident : & $data_type:ty| $e:expr ) => {
        {
            struct SomeDataRefAction { $( $v: $v_type ),* }

            impl DataRefAction<$data_type> for SomeDataRefAction {
                async fn execute (&self, $data : & $data_type) -> std::result::Result<(),OdinActionError> {
                    $( let $v = &self. $v;)*
                    map_action_err($e)
                }
            }
            impl std::fmt::Debug for SomeDataRefAction {
                fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "DataRefAction<{}>", stringify!($data_type))
                }
            }

            SomeDataRefAction{ $( $v: $v $(. $op () )? ),* }
        }
    }
}


/// an empty [`DataRefAction<T>`]. Transparent alternative for `Option<DataRefAction<T>>`
pub struct NoDataRefAction<T> where T: Send { _phantom: PhantomData<T> }

impl<T> DataRefAction<T> for NoDataRefAction<T> where T: Send {
    fn execute (&self, _data: &T) -> impl std::future::Future<Output = std::result::Result<(),OdinActionError>> + Send { ready(Ok(()) )}
}
impl<T> Debug for NoDataRefAction<T> where T: Send {
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoDataRefAction<{}>", abbrev_type_name::<T>())
    }
}
pub fn no_dataref_action<T>()->NoDataRefAction<T> where T: Send { NoDataRefAction { _phantom: PhantomData } }

/// a [`DataRefAction`] with a second `bidata` execute argument, which can be used to pass information
/// from the triggering request.
/// Note there is no corresponding `BiDynDataRefAction` as this normally would be a [`DynDataRefAction`]
/// that captures the bidata value from its definition site. `BiDataRefAction` is a way to avoid the associated
/// runtime cost if requester and owner both know the bidata type and the requester can pass it in through a message.
pub trait BiDataRefAction<T,A>: Debug + Send {
    fn execute (&self, data: &T, bidata: A) -> impl std::future::Future<Output = std::result::Result<(),OdinActionError>> + Send;
}

/// macro to define and instantiate ad hoc actions taking two data arguments (of which the first is a reference).
/// See [module] doc for general use and expansion.
#[macro_export]
macro_rules! bi_dataref_action {
    ( $( $v:ident $(. $op:ident ())? : $v_type:ty ),* => |$data:ident : & $data_type:ty, $bidata:ident: $bidata_type:ty| $e:expr ) => {
        {
            struct SomeBiDataRefAction { $( $v: $v_type ),* }

            impl BiDataRefAction<$data_type,$bidata_type> for SomeBiDataRefAction {
                async fn execute (&self, $data : & $data_type, $bidata : $bidata_type) -> std::result::Result<(),OdinActionError> {
                    $( let $v = &self. $v;)*
                    map_action_err($e)
                }
            }
            impl std::fmt::Debug for SomeBiDataRefAction {
                fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "SomeBiDataRefAction<{},{}>", stringify!($data_type),stringify!($bidata_type))
                }
            }

            SomeBiDataRefAction{ $( $v: $v $(. $op () )? ),* }
        }
    }
}


/// an empty [`BiDataRefAction<T,A>`]. Transparent alternative for `Option<BiDataRefAction<T,A>>`
pub struct NoBiDataRefAction<T,A>  where T: Send, A: Send { _phantom1: PhantomData<T>, _phantom2: PhantomData<A> }

impl<T,A> BiDataRefAction<T,A> for NoBiDataRefAction<T,A>  where T: Send, A: Send {
    fn execute (&self, _data: &T, _bidata: A) -> impl std::future::Future<Output = std::result::Result<(),OdinActionError>> + Send { ready(Ok(()) )}
}
impl<T,A> Debug for NoBiDataRefAction<T,A>  where T: Send, A: Send {
    fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoBiDataRefAction<{},{}>", abbrev_type_name::<T>(), abbrev_type_name::<A>())
    }
}

pub fn no_bi_dataref_action<T,A>()->NoBiDataRefAction<T,A>  where T: Send, A: Send { 
    NoBiDataRefAction { _phantom1: PhantomData, _phantom2: PhantomData } 
}

/// analoguous to the [`DynDataActionTrait<T>`] but executing with a `&T` data reference.
/// Just as `DynDataActionTrait` this is mostly used indirectly through its corresponding
/// [`DynDataRefAction<T>`] type
#[async_trait]
pub trait DynDataRefActionTrait<T>: Debug + Send + Sync {
    async fn execute (&self, data: &T) -> std::result::Result<(),OdinActionError>;
}

/// analoguous to [`DynDataAction<T>`] type but executing with a `&T` data reference
/// Note: this has per-execution runtime cost as the returned future needs to be pinboxed
pub type DynDataRefAction<T> = Box<dyn DynDataRefActionTrait<T>>; 

/// macro to define and instantiate ad hoc action types taking a reference argument, to be used
/// where action objects need to be [`Send`] and/or storable in homogenous containers (as trait objects).
/// See [module] doc for general use and expansion.
#[macro_export]
macro_rules! dyn_dataref_action {
    ( $( $v:ident $(. $op:ident ())? : $v_type:ty ),* => |$data:ident : & $data_type:ty| $e:expr ) => {
        {
            use async_trait::async_trait;

            struct SomeDynDataRefAction { $( $v: $v_type ),* }

            #[async_trait]
            impl odin_action::DynDataRefActionTrait<$data_type> for SomeDynDataRefAction {
                async fn execute (&self, $data : & $data_type) -> std::result::Result<(),OdinActionError> {
                    $( let $v = &self. $v;)*
                    map_action_err($e)
                }
            }
            impl std::fmt::Debug for SomeDynDataRefAction {
                fn fmt (&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "DynDataRefAction<{}>", stringify!($data_type))
                }
            }

            Box::new( SomeDynDataRefAction{ $( $v: $v $(. $op () )? ),* } )
        }
    }
}


/* #endregion DataRefAction */

/* #region dyn action lists *********************************************************************************/

/// container to store DynDataAction objects
pub struct DynDataActionList<T> where T: Clone {
    entries: Vec<DynDataAction<T>> 
}

impl <T> DynDataActionList<T> where T: Clone {
    pub fn new ()->Self { 
        DynDataActionList{ entries: Vec::new() } 
    }
    
    pub async fn execute (&self, data: T, ignore_err: bool) -> std::result::Result<(),OdinActionError> {
        if !self.is_empty() {
            let n = self.entries.len()-1;
            if ignore_err {
                for i in 0..n { let _ = self.entries[i].execute(data.clone()).await; }
                let _ = self.entries[n].execute(data).await;
            } else {
                for i in 0..n { self.entries[i].execute(data.clone()).await?; }
                self.entries[n].execute(data).await?;
            }
        }
        Ok(())
    }
}

impl <T> Deref for DynDataActionList<T> where T: Clone {
    type Target = Vec<DynDataAction<T>>;
    fn deref(& self) -> &Self::Target {
        &self.entries
    }
}

impl <T> DerefMut for DynDataActionList<T> where T: Clone {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}


/// container to store DynDataRefAction objects
pub struct DynDataRefActionList<T> where T: Send + Sync {
    entries: Vec<DynDataRefAction<T>> 
}

impl <T> DynDataRefActionList<T> where T: Send + Sync {
    pub fn new ()->Self { 
        DynDataRefActionList{ entries: Vec::new() }
    }

    pub async fn execute (&self, data: &T, ignore_err: bool) -> std::result::Result<(),OdinActionError> {
        if ignore_err {
            for e in &self.entries {
                let _ = e.execute(data).await;
            }
        } else {
            for e in &self.entries {
                e.execute(data).await?;
            }
        }
        Ok(())
    }
}

impl <T> Deref for DynDataRefActionList<T> where T: Send + Sync {
    type Target = Vec<DynDataRefAction<T>>;
    fn deref(& self) -> &Self::Target {
        &self.entries
    }
}

impl <T> DerefMut for DynDataRefActionList<T> where T: Send + Sync {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.entries
    }
}

/* #endregion dyn action lists */