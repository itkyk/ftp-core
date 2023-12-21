#![deny(clippy::all)]

#[macro_use]
mod ftp_module;

use napi_derive::napi;


#[napi]
fn deploy(local_root: String, remote_root: String, host: String,  password: String, user: String, port:String, is_delete: bool) {

    let str_port: &str = &port;
    let result_host = host.to_string() + ":" + &str_port;
    let mut deleting = false;
    if is_delete {
        deleting = true;
    }
    let _ = ftp_module::ftp_init(&local_root, &remote_root, &result_host.as_str(), &user, &password, deleting);
}
