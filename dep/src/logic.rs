use crate::error::{ApplicationError, LoadError};
use async_std::io::prelude::BufReadExt;
use async_std::io::{BufReader, BufWriter, ReadExt, WriteExt};
use async_std::prelude::*;
use mslnk::ShellLink;
use std::env::{self, current_dir, set_current_dir};
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::Command;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
use winreg::RegKey;

pub fn deploy_count_path() -> PathBuf {
    Path::new(r"pre\deploy_count.txt").into()
}
pub fn root_dir() -> PathBuf {
    Path::new("pre").into()
}
pub static ROOT_DIR: &str = "pre";
static MKCERT_PATH: &str = r"ssl certificate\mkcert-v1.4.1-windows-amd64.exe";
static CONF_PATH: &str = r"Settings.toml";
static ANKISYNCD_PATH: &str = r"pre\ankisyncd.exe";
static DEPLOY_COUNT_PATH: &str = r"pre\deploy_count.txt";
pub async fn async_read_to_string(path: &Path) -> Result<String, LoadError> {
    if !path.exists() {
        async_std::fs::File::create(path)
            .await
            .map_err(|_| LoadError::FileError)?;
    }
    let mut contents = String::new();
    let mut f = async_std::fs::File::open(path)
        .await
        .map_err(|_| LoadError::FileError)?;
    f.read_to_string(&mut contents)
        .await
        .map_err(|_| LoadError::FileError)?;
    Ok(contents.trim().to_owned())
}

/// read_pc_anki_ver from Windows register edit ->46
fn read_pc_anki_ver_from_regedit() -> Result<u8, ApplicationError> {
    // read anki ver 2.1.36
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let cur_ver =
        hklm.open_subkey(r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Anki")?;
    let ankiver: String = cur_ver.get_value("DisplayVersion")?;
    // split version to get minus ver 36
    let ver_s: Vec<&str> = ankiver.split(".").collect();
    let min_ver = ver_s.get(2).unwrap().to_string().parse::<u8>()?;
    Ok(min_ver)
}

/// if windows anki ver is above 2.1.10
pub fn pc_ver_required() -> bool {
    let ver_value = read_pc_anki_ver_from_regedit();
    match ver_value {
        Ok(x) if x > 10 => true,
        _ => false,
    }
}
/// ```
///    use winreg::enums::HKEY_CURRENT_USER;
/// use winreg::RegKey;
/// ```
/// winreg = "0.8"
/// # return Admin
pub fn output_pc_username() -> Result<String, ApplicationError> {
    let hklm = RegKey::predef(HKEY_CURRENT_USER);
    let cur_ver = hklm.open_subkey("Volatile Environment")?;
    let username: String = cur_ver.get_value("USERNAME")?;
    Ok(username)
}
async fn copy_addon(ankisyncd_dir: PathBuf) -> Result<(), ApplicationError> {
    let cwd = env::current_dir()?;

    // create dir if not exist
    if !ankisyncd_dir.exists() {
        async_std::fs::create_dir(&ankisyncd_dir).await?;
    }

    // read files names from deployer's addon dir
    let server_addon_dir = cwd.join(r"pre\addon\ankisyncd");
    let mut entries = async_std::fs::read_dir(&server_addon_dir).await?;
    while let Some(res) = entries.next().await {
        let entry = res?.path();
        let f = entry.file_name();
        let dst_path = ankisyncd_dir.join(f.unwrap());

        async_std::fs::copy(entry, &dst_path).await?;
    }
    Ok(())
}
async fn set_pcip(ankisyncd_dir: PathBuf, ipaddr: &str) -> Result<(), ApplicationError> {
    let conf_file = ankisyncd_dir.join("config.json");
    let b = Vec::new();
    let f = async_std::fs::File::open(conf_file.clone()).await?;
    let mut lines = BufReader::new(f).lines();

    let mut buf = BufWriter::new(b);
    while let Some(line) = lines.next().await {
        let l = line?;
        let cont = if l.contains("syncaddr") {
            format!("\"syncaddr\":\"https://{}:27701/\"", &ipaddr)
        } else {
            l
        };
        buf.write(cont.as_bytes()).await?;
    }

    let mut ff = async_std::fs::File::create(conf_file).await?;
    ff.write_all(buf.buffer()).await?;

    Ok(())
}
/// copy addon files from deployer dir to PC Anki add-on dir
///
/// modify addon file 's sync address
pub async fn pcip_modify(ipaddr: &str) -> Result<(), ApplicationError> {
    let pc_usrname = output_pc_username()?;
    let addon_dir = Path::new(r"C:\Users")
        .join(&pc_usrname)
        .join(r"AppData\Roaming\Anki2\addons21");
    let ankisyncd_dir = addon_dir.join("ankisyncd");

    copy_addon(ankisyncd_dir.clone()).await?;
    set_pcip(ankisyncd_dir, ipaddr).await?;
    Ok(())
}
pub fn output_user_profile() -> Result<String, ApplicationError> {
    // return
    // userprofile:C:\\Users\\Admin
    let hklm = RegKey::predef(HKEY_CURRENT_USER);
    let cur_ver = hklm.open_subkey("Volatile Environment")?;
    let userprofile: String = cur_ver.get_value("USERPROFILE")?;
    Ok(userprofile)
}
/// run external command mkcert to import rootCA to system trusted store
///
///  send rootCA.crt to desktop
pub async fn import_to_sysstore() -> Result<(), ApplicationError> {
    set_current_dir(&ROOT_DIR)?;
    let cmd = MKCERT_PATH;
    Command::new(cmd)
        .arg("-install")
        .output()
        .expect("install CA");

    set_current_dir(current_dir()?.parent().unwrap())?;

    let usrname_profile = output_user_profile()?;

    let rootca_file_path = Path::new(&usrname_profile).join(r"AppData\Local\mkcert\rootCA.pem");

    let rootca_desktop_path = desktop()?.join("rootCA.crt");

    async_std::fs::copy(rootca_file_path, rootca_desktop_path).await?;

    Ok(())
}
/// run cmd mkcert to gen server cert and key files
///
/// write their paths to file Settings.toml
pub async fn install_servcerts(ipaddr: &str) -> Result<(), ApplicationError> {
    set_current_dir(&ROOT_DIR)?;

    let cmd = MKCERT_PATH;
    Command::new(cmd)
        .args(&["localhost", "127.0.0.1", "::1", ipaddr])
        .status()
        .expect("gen server certs");
    // add Anki env variable to PATH
    Command::new("setx")
        .args(&["ANKI_NOVERIFYSSL", "1"])
        .status()
        .unwrap();

    set_ssl().await?;

    set_current_dir(current_dir()?.parent().unwrap())?;

    Ok(())
}
// create shortcut to ankisyncd.exe and send it to desktop
pub async fn send_shortcut() -> Result<(), ApplicationError> {
    let target = current_dir()?.join(ANKISYNCD_PATH);
    let lnk = desktop()?.join("anki_server.lnk");
    let sl = ShellLink::new(target).unwrap();
    sl.create_lnk(lnk).unwrap();

    Ok(())
}
pub async fn read_parse_deploy_count() -> Result<u8, ApplicationError> {
    let p = Path::new(&DEPLOY_COUNT_PATH);
    let mut s = String::new();
    if !p.exists() {
        async_std::fs::File::create(p).await?;
    }
    let f = async_std::fs::File::open(p).await?;
    BufReader::new(f).read_to_string(&mut s).await?;

    let deploy_count = if s.trim().is_empty() {
        0
    } else {
        s.parse::<u8>()?
    };
    Ok(deploy_count)
}
/// read and parse file contents from file deploy_count
///
/// plus one write to it
async fn deploy_count_plusone() -> Result<(), ApplicationError> {
    let count = read_parse_deploy_count().await? + 1;
    let p = Path::new(&DEPLOY_COUNT_PATH);

    let mut f = async_std::fs::File::create(p).await?;
    f.write(count.to_string().as_bytes()).await?;
    Ok(())
}
/// create folder in Windows start menu dir
///
/// copy shortcut from desktop to created dir
/// C:\Users\Admin\AppData\Roaming\Microsoft\Windows\Start Menu\Programs
pub async fn add_startmenu() -> Result<(), ApplicationError> {
    let lnk = desktop()?.join("anki_server.lnk");

    let menu_path = Path::new(&output_user_profile()?)
        .join(r"AppData\Roaming\Microsoft\Windows\Start Menu\Programs")
        .join("anki_server.lnk");
    if !menu_path.exists() {
        async_std::fs::copy(lnk, &menu_path).await?;
    }
    deploy_count_plusone().await?;
    Ok(())
}
async fn set_ssl() -> Result<(), ApplicationError> {
    let mut cert = None;
    let mut key = None;
    let mut entries = async_std::fs::read_dir(current_dir()?).await?;
    while let Some(res) = entries.next().await {
        let entry = res?.path();
        if format!("{}", entry.display()).contains(".pem") {
            if entry.to_str().unwrap().contains("key") {
                key = Some(entry.file_name().unwrap().to_str().unwrap().to_owned())
            } else {
                cert = Some(entry.file_name().unwrap().to_str().unwrap().to_owned())
            }
        }
    }

    let conf_file = Path::new(CONF_PATH);
    let b = Vec::new();
    let f = async_std::fs::File::open(conf_file.clone()).await?;
    let mut lines = BufReader::new(f).lines();

    let mut buf = BufWriter::new(b);
    while let Some(line) = lines.next().await {
        let l = line?;
        let mut cont = if l.contains("ssl_enable") {
            "ssl_enable=true".into()
        } else if l.contains("cert_file") {
            format!("cert_file=\"{}\"", cert.clone().unwrap())
        } else if l.contains("key_file") {
            format!("key_file=\"{}\"", key.clone().unwrap())
        } else {
            l
        };
        cont.push('\n');
        buf.write(cont.as_bytes()).await?;
    }

    let mut ff = async_std::fs::File::create(conf_file).await?;
    ff.write_all(buf.buffer()).await?;

    Ok(())
}
fn desktop() -> Result<PathBuf, ApplicationError> {
    let usrname_profile = output_user_profile()?;
    Ok(Path::new(&usrname_profile).join("Desktop").into())
}
/// lookup ip lan addr
pub fn lookup_ip() -> Result<String, ApplicationError> {
    // look up local ipaddr

    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket
        .connect("8.8.8.8:80")
        .expect("Couldn't connect to the server...");
    let ipaddr = socket.local_addr().unwrap().ip();
    let ipaddr_str = format!("{}", ipaddr);

    Ok(ipaddr_str)
}
pub async fn lsusr() -> Result<String, ApplicationError> {
    set_current_dir(&ROOT_DIR)?;

    let cmd = r"ankisyncd.exe";

    let out = Command::new(cmd)
        .args(&["user", "-l"])
        .output()
        .expect("run adduser");

    let s = String::from_utf8(out.stdout)?.trim().to_owned();
    let v = s.split("\n").collect::<Vec<_>>().join(" ");
    set_current_dir(current_dir().unwrap().parent().unwrap())?;
    Ok(v)
}

pub fn delusr(name: String) -> Result<(), ApplicationError> {
    set_current_dir(&ROOT_DIR).unwrap();
    let cmd = r"ankisyncd.exe";
    if !name.is_empty() {
        Command::new(cmd)
            .args(&["user", "-d", &name])
            .status()
            .expect("run adduser");
    }

    set_current_dir(current_dir()?.parent().unwrap())?;
    Ok(())
}
pub fn chgepass(name: String, pass: String) -> Result<(), ApplicationError> {
    set_current_dir(&ROOT_DIR).unwrap();
    let cmd = r"ankisyncd.exe";
    if !(name.is_empty() && pass.is_empty()) {
        Command::new(cmd)
            .args(&["user", "-p", &name, &pass])
            .status()
            .expect("run adduser");
    }

    set_current_dir(current_dir()?.parent().unwrap())?;
    Ok(())
}

pub fn addusr(name: String, pass: String) -> Result<(), ApplicationError> {
    set_current_dir(&ROOT_DIR).unwrap();

    let cmd = r"ankisyncd.exe";
    if !(name.is_empty() && pass.is_empty()) {
        Command::new(cmd)
            .args(&["user", "-a", &name, &pass])
            .status()
            .expect("run adduser");
    }

    set_current_dir(current_dir()?.parent().unwrap())?;
    Ok(())
}
