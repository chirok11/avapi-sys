use avapi_sys::IOTC;

fn main() {
    // first argument is UID
    // second argument is channel_id
    let args: Vec<String> = std::env::args().collect();
    let uid = args[1].clone();
    let channel_id = args[2].parse::<i32>().unwrap();

    let iotc = IOTC::new(32);
    assert!(iotc.is_ok());
    let mut iotc = iotc.unwrap();
    iotc.connect_to(uid);
    iotc.start_av("admin".into(), "".into(), channel_id);
    iotc.start_stream();
    // iotc.stop();
    iotc.video_frames();
}
