extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use core::fmt::Write;
use uefi::boot;
use uefi::proto::media::file::{Directory, File, FileAttribute, FileMode};
use uefi::CString16;

fn open_volume() -> Result<Directory, String> {
    let handle = boot::image_handle();
    let mut fs =
        boot::get_image_file_system(handle).map_err(|_| String::from("Failed to open FS"))?;
    fs.open_volume()
        .map_err(|_| String::from("Failed to open volume"))
}

fn to_uefi_path(path: &str) -> Result<CString16, String> {
    let converted: String = path.replace('/', "\\");
    CString16::try_from(converted.as_str()).map_err(|_| String::from("Invalid path"))
}

pub fn cmd_ls(args: &str) -> Result<String, String> {
    let mut root = open_volume()?;
    let path = if args.is_empty() { "\\" } else { args };
    let path_cstr = to_uefi_path(path)?;

    let handle = root
        .open(&path_cstr, FileMode::Read, FileAttribute::empty())
        .map_err(|_| format!("Cannot open '{}'", path))?;

    let mut dir = handle
        .into_directory()
        .ok_or_else(|| format!("'{}' is not a directory", path))?;

    let mut output = String::new();
    loop {
        match dir.read_entry_boxed() {
            Ok(Some(info)) => {
                let name = info.file_name();
                if info.is_directory() {
                    let _ = writeln!(output, "  <DIR>  {}", name);
                } else {
                    let _ = writeln!(output, "  {:>8}  {}", info.file_size(), name);
                }
            }
            Ok(None) => break,
            Err(_) => return Err(String::from("Error reading directory")),
        }
    }
    Ok(output)
}

pub fn cmd_cat(args: &str) -> Result<String, String> {
    if args.is_empty() {
        return Err(String::from("Usage: cat <file>"));
    }
    let mut root = open_volume()?;
    let path_cstr = to_uefi_path(args)?;

    let handle = root
        .open(&path_cstr, FileMode::Read, FileAttribute::empty())
        .map_err(|_| format!("Cannot open '{}'", args))?;

    let mut file = handle
        .into_regular_file()
        .ok_or_else(|| format!("'{}' is a directory", args))?;

    let mut buf = vec![0u8; 4096];
    let mut content = String::new();
    loop {
        let n = file.read(&mut buf).map_err(|_| String::from("Read error"))?;
        if n == 0 {
            break;
        }
        content.push_str(&String::from_utf8_lossy(&buf[..n]));
    }
    Ok(content)
}

pub fn cmd_write(args: &str) -> Result<String, String> {
    let (filename, text) = args
        .split_once(' ')
        .ok_or_else(|| String::from("Usage: write <file> <text>"))?;

    let mut root = open_volume()?;
    let path_cstr = to_uefi_path(filename)?;

    let handle = root
        .open(
            &path_cstr,
            FileMode::CreateReadWrite,
            FileAttribute::empty(),
        )
        .map_err(|_| format!("Cannot create '{}'", filename))?;

    let mut file = handle
        .into_regular_file()
        .ok_or_else(|| format!("'{}' is a directory", filename))?;

    file.write(text.as_bytes())
        .map_err(|_| String::from("Write error"))?;

    Ok(format!("Wrote {} bytes to {}", text.len(), filename))
}

pub fn cmd_mkdir(args: &str) -> Result<String, String> {
    if args.is_empty() {
        return Err(String::from("Usage: mkdir <dir>"));
    }
    let mut root = open_volume()?;
    let path_cstr = to_uefi_path(args)?;

    let _handle = root
        .open(
            &path_cstr,
            FileMode::CreateReadWrite,
            FileAttribute::DIRECTORY,
        )
        .map_err(|_| format!("Cannot create directory '{}'", args))?;

    Ok(format!("Created directory: {}", args))
}

pub fn cmd_rm(args: &str) -> Result<String, String> {
    if args.is_empty() {
        return Err(String::from("Usage: rm <file>"));
    }
    let mut root = open_volume()?;
    let path_cstr = to_uefi_path(args)?;

    let handle = root
        .open(&path_cstr, FileMode::ReadWrite, FileAttribute::empty())
        .map_err(|_| format!("Cannot open '{}'", args))?;

    handle
        .delete()
        .map_err(|_| format!("Cannot delete '{}'", args))?;

    Ok(format!("Deleted: {}", args))
}
