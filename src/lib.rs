use crate::bindings::{
    avClientStart2, avGetAVApiVer, avInitialize, avRecvFrameData2, avSendIOCtrl,
    AVIOCtrlType_IOTYPE_INNER_SND_DATA_DELAY, FRAMEINFO_t, IOTC_Connect_ByUID,
    IOTC_Connect_ByUID_Parallel, IOTC_Get_SessionID, IOTC_Get_Version, IOTC_Initialize,
    IOTC_Session_Check, SMsgAVIoctrlAVStream, AV_ER_DATA_NOREADY, AV_ER_INCOMPLETE_FRAME,
    AV_ER_LOSED_THIS_FRAME, AV_ER_REMOTE_TIMEOUT_DISCONNECT, AV_ER_SESSION_CLOSE_BY_REMOTE,
    ENUM_AVIOCTRL_MSGTYPE_IOTYPE_USER_IPCAM_START, IOTC_ER_INVALID_SID,
};
use anyhow::bail;
use bindings::{avClientStop, avDeInitialize, st_SInfo, IOTC_DeInitialize, IOTC_Session_Close};
use std::ffi::CString;
use std::fs::OpenOptions;
use std::io::Write;
use std::mem::{zeroed, MaybeUninit};
use std::os::raw::{c_char, c_int, c_uchar};
use std::time::Duration;

#[allow(dead_code)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
mod bindings;

pub struct IOTC {
    sid: i32,
    av_index: i32,
}

type Error = anyhow::Error;

impl IOTC {
    pub fn new(channels: i32) -> Result<Self, Error> {
        let result = unsafe {
            IOTC_Initialize(
                0,
                str_to_c_str("m1.iotcplatform.com"),
                str_to_c_str("m2.iotcplatform.com"),
                str_to_c_str("m4.iotcplatform.com"),
                str_to_c_str("m5.iotcplatform.com"),
            )
        };

        if result != 0 {
            bail!("IOTC_Initialize failed");
        }

        unsafe { avInitialize(channels) };

        let mut iotc_version = 0;
        unsafe { IOTC_Get_Version(&mut iotc_version) };
        let av_ver = unsafe { avGetAVApiVer() };

        println!("IOTC version: {}", iotc_version);
        println!("AV version: {}", av_ver);

        Ok(IOTC {
            sid: -1,
            av_index: -1,
        })
    }

    pub fn connect_to(&mut self, uid: String) {
        unsafe {
            self.sid = IOTC_Get_SessionID();
            println!("IOTC_Get_SessionID: {}", self.sid);

            IOTC_Connect_ByUID_Parallel(str_to_c_str(&uid), self.sid);
            println!("IOTC_Connect_ByUID: {}", self.sid);
        }
    }

    pub fn start_stream(&mut self) {
        unsafe {
            let v = 0u16;
            let ret = avSendIOCtrl(
                self.av_index,
                AVIOCtrlType_IOTYPE_INNER_SND_DATA_DELAY,
                v as *const c_char,
                2,
            );
            println!("{}", ret);
            assert!(ret >= 0);
            println!("avSendIOCtrl data delay: {}", ret);
            let stream = SMsgAVIoctrlAVStream {
                channel: 0,
                ..zeroed()
            };
            let ret = avSendIOCtrl(
                self.av_index,
                ENUM_AVIOCTRL_MSGTYPE_IOTYPE_USER_IPCAM_START,
                &stream as *const SMsgAVIoctrlAVStream as *const c_char,
                std::mem::size_of::<SMsgAVIoctrlAVStream>() as i32,
            );
            assert!(ret >= 0);
            println!("avSendIOCtrl start IPCAM: {}", ret);
        }
    }

    pub fn start_av(&mut self, username: String, password: String, channel_id: i32) {
        unsafe {
            let mut serv_type = 0;
            let mut pn: c_int = -1;
            let av_index = avClientStart2(
                self.sid,
                str_to_c_str(&username),
                str_to_c_str(&password),
                20,
                &mut serv_type,
                channel_id as c_uchar,
                &mut pn,
            );
            println!("av_index: {}", av_index);
            self.av_index = av_index;

            let mut sinfo: MaybeUninit<st_SInfo> = MaybeUninit::uninit().assume_init();

            let c = IOTC_Session_Check(self.sid, sinfo.as_mut_ptr());
            println!("IOTC_Session_Check: {}", c);

            assert_eq!(c, 0);
        }
    }

    pub fn stop(&self) {
        unsafe {
            avClientStop(self.av_index);
            IOTC_Session_Close(self.sid);
        }
    }

    pub fn video_frames(&self) {
        let mut fs = OpenOptions::new()
            .create(true)
            .write(true)
            .open("video.mp4")
            .unwrap();

        let mut fps = 0;

        unsafe {
            loop {
                let mut actual_frame_size = 0;
                let mut expected_frame_size = 0;
                let mut actual_frame_info_size = 0;
                let mut frame_idx = 0;
                let mut buf = vec![0u8; 2304000]; // 128000 = 1024 * 1024
                let mut frame_info: MaybeUninit<FRAMEINFO_t> =
                    unsafe { MaybeUninit::uninit().assume_init() };

                let ret = avRecvFrameData2(
                    self.av_index,
                    buf.as_mut_ptr() as *mut i8,
                    2304000,
                    &mut actual_frame_size,
                    &mut expected_frame_size,
                    frame_info.as_mut_ptr() as *mut i8,
                    16,
                    &mut actual_frame_info_size,
                    &mut frame_idx,
                );

                println!("ret: {}", ret);

                match ret {
                    AV_ER_DATA_NOREADY => {
                        // std::thread::sleep(Duration::from_micros(10 * 1000));
                        std::thread::sleep(Duration::from_secs(1));
                        println!("fps: {}", fps);
                        fps = 0;
                    }
                    AV_ER_LOSED_THIS_FRAME => {
                        println!("AV_ER_LOSED_THIS_FRAME");
                    }
                    AV_ER_INCOMPLETE_FRAME => {
                        println!("AV_ER_INCOMPLETE_FRAME");
                    }
                    AV_ER_SESSION_CLOSE_BY_REMOTE => {
                        println!("AV_ER_SESSION_CLOSE_BY_REMOTE");
                        break;
                    }
                    AV_ER_REMOTE_TIMEOUT_DISCONNECT => {
                        println!("AV_ER_REMOTE_TIMEOUT_DISCONNECT");
                        break;
                    }
                    IOTC_ER_INVALID_SID => {
                        println!("IOTC_ER_INVALID_SID");
                        break;
                    }
                    ret => {
                        if ret > 0 {
                            println!("ret: {}", ret);
                            println!("actual_frame_size: {}", actual_frame_size);
                            println!("expected_frame_size: {}", expected_frame_size);
                            println!("actual_frame_info_size: {}", actual_frame_info_size);
                            println!("frame_idx: {}", frame_idx);

                            let n = fs.write(&buf[..ret as usize]).unwrap();
                            fps += 1;
                            assert_eq!(n, ret as usize);
                            fs.flush().expect("failed to flush file");
                        }
                    }
                    _ => {
                        println!("ret: {}", ret);
                    }
                }
            }
        }
    }
}

impl Drop for IOTC {
    fn drop(&mut self) {
        unsafe {
            avDeInitialize();
            IOTC_DeInitialize();
        }
    }
}

fn str_to_c_str(s: &str) -> *const c_char {
    CString::new(s).unwrap().into_raw()
}
