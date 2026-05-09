

# RustDirLister

## Changes

- UI fixes
- askama replaced with tera, so you can modify html without rebuild 


**RustDirLister** is a lightweight web application built with Rust and Axum that provides an interface for browsing the contents of directories on a server. The project allows displaying files and folders with sorting, icons, and basic navigation, and supports customization of the starting directory and service name.

<p align="center">
<img src="https://github.com/Kerimniy/RustDirLister/blob/v0.1/for_readme/preview.png" alt="logo" width="90%">
</p>

### Key Features
- Browse directory contents.
- Support for folder navigation via the web interface.
- **Display of `README.md`**: If a `README.md` file exists in the directory, its contents are displayed at the top of the page.
- Configurable starting directory via the `PATH` variable.

## Configure the application:

   - Create "PATH" file and write serving folder
   - Create "USER" file and write login and password like "your_login your_pwd", then you will need to auth to see folder
   - Create "HOST" file and write ipv4 and port. e.g 127.0.0.1:8000 (default)


---

