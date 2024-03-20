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

 // items that abstract flume MPSC channels
// note this get conditionally included into the respective runtime module 

use flume::{ bounded, Sender, Receiver, TrySendError, TryRecvError };

pub type MpscSender<M> = Sender<M>;
pub type MpscReceiver<M> =Receiver<M>;

#[inline] 
pub fn create_mpsc_sender_receiver <MsgType> (bound: usize) -> (MpscSender<MsgType>,MpscReceiver<MsgType>)
    where MsgType: Send
{
    bounded::<MsgType>(bound)
}

#[inline] 
fn is_closed<M> (tx: &MpscSender<M>)->bool { 
    tx.is_disconnected() 
}

#[inline] 
fn send<M> (tx: &MpscSender<M>, msg: M)->SendFuture<'_,M> { 
    tx.send_async(msg)
}

#[inline] 
fn recv<M> (tx: &MpscReceiver<M>)->ReceiveFuture<'_,M> { 
    tx.recv_async()
}

macro_rules! match_try_send {
    ($sender:expr, $msg:expr, ok => $ok_blk:block full => $full_blk:block closed => $closed_blk:block) => {
        match $sender.try_send($msg) {
            Ok(()) => $ok_blk
            Err(TrySendError::Full(_)) => $full_blk
            Err(TrySendError::Disconnected(_)) => $closed_blk
        }
    }
}