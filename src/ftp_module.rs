use ftp::{FtpStream};
use std::fs;
use std::fs::File;
use std::io::{Cursor, Read};
use std::option::Option::Some;
use std::path::Path;
use std::path::PathBuf;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Result;

struct DeletePathMap {
    file_type: String,
    file_name: String,
}


impl Hash for DeletePathMap {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.file_type.hash(state);
        self.file_name.hash(state);
    }
}


fn get_remotes(target: &str, dir: &PathBuf) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
    let mut paths = Vec::new();
    let mut dirs = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let (result_files, result_dir) = &mut get_remotes(&target, &path)?;
                paths.append(result_files);
                let vec_dir = &mut Vec::new();
                vec_dir.push(path.strip_prefix(Path::new(&target)).unwrap().to_path_buf());
                dirs.append(vec_dir);
                dirs.append(result_dir);
            } else {
                let entry_path = &entry.path();
                let p = entry_path.to_str().unwrap();
                let res = Path::new(&p).strip_prefix(Path::new(&target));
                if res.is_ok() {
                    let buf_path = res.unwrap().to_path_buf();
                    paths.push(buf_path);
                }
            }
        }
    }
    Ok((paths, dirs))
}

fn create_dirs (mut ftp: FtpStream, dir_list: Vec<PathBuf>) -> FtpStream {
    for dir in dir_list {
        ftp.mkdir(dir.to_str().unwrap()).ok();
    }
    return ftp;
}

pub fn upload_files(mut ftp: FtpStream, local: &str) -> Result<FtpStream> {
    let entries_path = PathBuf::from(&local);
    let (files, dirs) = get_remotes(&local, &entries_path).ok().unwrap();
    ftp = create_dirs(ftp, dirs);
    let mut count: u64 = 0;
    for _ in &files {
        count+=1;
    }
    let bar = ProgressBar::new(count);
    bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .unwrap()
        .progress_chars("##-"));
    for file in files {
        let mut full_path = PathBuf::new();
        full_path.push(&local);
        full_path.push(&file.to_str().unwrap());
        let mut error_message = String::from("Cannot Found ( ");
        error_message.push_str(&full_path.to_str().unwrap());
        error_message.push_str(" )");
        let mut file_data = File::open(&full_path).expect(error_message.as_str());
        let mut buffer = Vec::new();
        file_data.read_to_end(&mut buffer).unwrap();
        let mut renderer = Cursor::new(buffer.as_slice());
        let file_path = &file.to_str().unwrap();
        ftp.put(file_path, &mut renderer).unwrap();
        bar.set_message(file_path.to_string());
        bar.inc(1);
    }
    bar.finish();
    return Ok(ftp);
}

fn get_delete_files(mut ftp: FtpStream, root: &str) -> (FtpStream, Vec<DeletePathMap>){
    let mut files: Vec<DeletePathMap> = Vec::new();
    let file_list = ftp.list(Some(root)).unwrap();
    for file in file_list {
        let arr_file_data = &file.split_whitespace().collect::<Vec<&str>>();
        let file_2 = &file.clone();
        let file_1 = &arr_file_data[arr_file_data.len() - 1];
        let file_data = DeletePathMap{
            file_type: file_2[..1].to_string(),
            file_name: file_1.to_string()
        };
        files.push(file_data);
    }
    return (ftp, files);
}

fn delete (mut ftp: FtpStream, path: &str) -> FtpStream {
    ftp.rm(&path).unwrap();
    return ftp;
}

fn delete_files (mut ftp: FtpStream, root: &str) -> FtpStream {
    let (f, file_list) = get_delete_files(ftp, root);
    ftp = f;
    for item in file_list {
        let mut delete_path = DefaultHasher::new();
        item.hash(&mut delete_path);
        delete_path.finish();
        if item.file_type != String::from("d") {
            // ファイルだったら
            let mut path = PathBuf::from(root);
            path.push(item.file_name);
            ftp = delete(ftp, path.to_str().unwrap());
        } else {
            // Directoryだったら
            let mut path = PathBuf::from(root);
            path.push(&item.file_name);
            let f = delete_files(ftp, &path.to_str().unwrap());
            ftp = f;
            ftp.rmdir(&path.to_str().unwrap()).ok();
        }
    };
    return ftp;
}

pub fn ftp_init(local: &str, remote: &str, host: &str, user: &str, pw: &str, is_delete: bool) -> Result<()> {
    let mut ftp = FtpStream::connect(host).unwrap();
    let _ = ftp.login(user, pw).unwrap();
    ftp.transfer_type(ftp::types::FileType::Binary).ok();
    for remote_root in remote.split("/").collect::<Vec<_>>() {
        let stream = ftp.size(&remote_root).is_ok();
        if stream == false {
            ftp.mkdir(&remote_root).ok();
        }
        ftp.cwd(remote_root).unwrap();
    }
    if is_delete {
        println!("Start delete remote");
        let last_delete_root = PathBuf::from("./");
        let _ftp = delete_files(ftp, &last_delete_root.to_str().unwrap());
        ftp = _ftp;
        println!("Finish delete remote");
    }
    println!("Start Upload");
    ftp = upload_files(ftp, local)?;
    println!("End Upload");
    ftp.quit().ok();
    Ok(())
}
