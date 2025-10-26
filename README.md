<p align="center">
<img src="https://github.com/Kerimniy/FlaskDirLister/blob/main/for_readme/full-logo.webp" alt="logo" width="66%">
</p>

**На русском тута ->** <a href="https://github.com/Kerimniy/FlaskDirLister/blob/main/README-RU.md">тык</a>

# FlaskDirLister

**FlaskDirLister** is a lightweight web application built with Python and Flask that provides an interface for browsing the contents of directories on a server. The project allows displaying files and folders with sorting, icons, and basic navigation, and supports customization of the starting directory and service name.

<p align="center">
<img src="https://github.com/Kerimniy/FlaskDirLister/blob/main/for_readme/preview.png" alt="logo" width="90%">
</p>

### Key Features
- Browse directory contents.
- Support for folder navigation via the web interface.
- **Display of `README.md`**: If a `README.md` file exists in the directory, its contents are displayed at the top of the page.
- Configurable starting directory via the `PATH` variable.
- Custom service name via the `SERVICENAME` variable (defaults to the server domain).

### Installation

1. **Requirements**:
   - Python 3.12+

2. **Clone the repository**:
   ```bash
   git clone https://github.com/Kerimniy/FlaskDirLister.git
   cd FlaskDirLister
   ```

3. **Install dependencies**:
   ```bash
   pip install -r requirements.txt
   ```

4. **Configure the application**:
   In the `app.py` file or a configuration file, set the following variables:
   ```python
   PATH = os.path.dirname(os.path.realpath(__file__)).replace("\\", "/") + "/static"
   SERVICENAME = None
   ```
   - **`PATH`**: Specifies the starting directory for displaying files. By default, it uses the `static` folder in the project root. You can change it to any other directory, for example:
     ```python
     PATH = "/home/user/my_files"
     ```
     **Important**: Ensure the path is secure and prevents access to system files (protection against directory traversal).
   - **`SERVICENAME`**: The service name displayed in the interface. If set to `None`, the server domain (e.g., `localhost` or `example.com`) is used. Example:
     ```python
     SERVICENAME = "FileDump"
     ```

5. **Run the application**:
   ```bash
   python app.py
   ```
   The application will be available at `http://localhost:5000` (for testing).

### Usage Example
1. Start the application.
2. Open a browser and navigate to `http://localhost:5000/`.
3. If the directory specified in `PATH` contains a `README.md` file, its contents will be displayed at the top of the page in HTML format.
4. Below, you will see a list of files and folders. Click on a folder to navigate into it or on a file to download it.

### Project Structure
```
FlaskDirLister/
├── app.py              # Main application file
├── templates/          # Jinja templates
├── static/            # Static files (CSS, JS, icons)
└── requirements.txt    # Dependencies
```

### Deployment
- For deployment, use `gunicorn`:
  ```bash
  pip install gunicorn
  gunicorn -w 4 -b 0.0.0.0:8000 app:app
  ```


### Contribution
Contributions are welcome! Create issues or pull requests on GitHub. Possible improvements include:
- File search functionality.
- User authentication.
- Support for file uploads.

---

