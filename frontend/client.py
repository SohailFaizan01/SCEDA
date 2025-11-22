import sys
import requests
import json
import base64
from pathlib import Path
from PySide6.QtWidgets import (
    QApplication, QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QPushButton, QLabel, QLineEdit, QTextEdit, QFileDialog, QMessageBox, QComboBox
)
from PySide6.QtCore import Qt
import qdarktheme
from cache import ProjectCache
from models import Project, IDEF0Block, Component

def get_backend_url():
    """Auto-detect backend URL"""
    try:
        r = requests.get("http://localhost:8080/health", timeout=2)
        if r.status_code == 200:
            print("✓ Backend found at localhost:8080")
            return "http://localhost:8080"
    except:
        pass
    
    print("✗ Backend not accessible at localhost:8080")
    return "http://localhost:8080"

class EDAClient(QMainWindow):
    def __init__(self):
        super().__init__()
        self.server_url = get_backend_url()
        self.current_user = "dev@eda-platform.local"
        self.current_project_id = None
        self.current_theme = "dark"
        self.cache = ProjectCache(self.server_url)  # Initialize cache
        self.init_ui()
        self.check_connection()
        
    def init_ui(self):
        self.setWindowTitle("EDA Platform - Client")
        self.setGeometry(100, 100, 1000, 700)
        
        main_widget = QWidget()
        self.setCentralWidget(main_widget)
        layout = QVBoxLayout(main_widget)
        
        # Theme selector at the top
        theme_layout = QHBoxLayout()
        theme_label = QLabel("🎨 Theme:")
        theme_label.setStyleSheet("font-weight: bold; font-size: 11pt;")
        theme_layout.addWidget(theme_label)
        
        self.theme_combo = QComboBox()
        self.theme_combo.addItems(["Dark", "Light", "Auto"])
        self.theme_combo.setCurrentText("Dark")
        self.theme_combo.currentTextChanged.connect(self.change_theme)
        self.theme_combo.setFixedWidth(150)
        theme_layout.addWidget(self.theme_combo)
        theme_layout.addStretch()
        
        layout.addLayout(theme_layout)
        
        # Connection status
        self.status_label = QLabel(f"Connecting to {self.server_url}...")
        self.status_label.setStyleSheet("padding: 10px; font-size: 12pt; border-radius: 5px;")
        layout.addWidget(self.status_label)
        
        # Project section
        project_label = QLabel("📁 PROJECT MANAGEMENT")
        project_label.setStyleSheet("font-weight: bold; font-size: 14pt; margin-top: 10px;")
        layout.addWidget(project_label)
        
        self.project_name_input = QLineEdit()
        self.project_name_input.setPlaceholderText("Enter project name...")
        self.project_name_input.setStyleSheet("padding: 8px; font-size: 11pt;")
        layout.addWidget(self.project_name_input)
        
        create_btn = QPushButton("Create Project")
        create_btn.setStyleSheet("padding: 10px; font-size: 11pt;")
        create_btn.clicked.connect(self.create_project)
        layout.addWidget(create_btn)
        
        # Tools section
        tools_label = QLabel("🔧 SIMULATION TOOLS")
        tools_label.setStyleSheet("font-weight: bold; font-size: 14pt; margin-top: 20px;")
        layout.addWidget(tools_label)
        
        list_tools_btn = QPushButton("List Available Tools")
        list_tools_btn.setStyleSheet("padding: 10px; font-size: 11pt;")
        list_tools_btn.clicked.connect(self.list_tools)
        layout.addWidget(list_tools_btn)
        
        # File section
        files_label = QLabel("📤 FILE UPLOAD")
        files_label.setStyleSheet("font-weight: bold; font-size: 14pt; margin-top: 20px;")
        layout.addWidget(files_label)
        
        upload_btn = QPushButton("Upload File to Current Project")
        upload_btn.setStyleSheet("padding: 10px; font-size: 11pt;")
        upload_btn.clicked.connect(self.upload_file)
        layout.addWidget(upload_btn)
        
        # Response display
        response_label = QLabel("📋 RESPONSE LOG")
        response_label.setStyleSheet("font-weight: bold; font-size: 14pt; margin-top: 20px;")
        layout.addWidget(response_label)
        
        self.response_display = QTextEdit()
        self.response_display.setReadOnly(True)
        self.response_display.setStyleSheet("font-family: 'Consolas', 'Courier New'; font-size: 10pt;")
        layout.addWidget(self.response_display)
    
    def change_theme(self, theme_name):
        """Change application theme"""
        theme_name = theme_name.lower()
        self.current_theme = theme_name
        
        app = QApplication.instance()
        if theme_name == "dark":
            app.setStyleSheet(qdarktheme.load_stylesheet("dark"))
        elif theme_name == "light":
            app.setStyleSheet(qdarktheme.load_stylesheet("light"))
        elif theme_name == "auto":
            app.setStyleSheet(qdarktheme.load_stylesheet("auto"))
        
        # Update status label colors
        if "Connected" in self.status_label.text():
            self.update_status_connected()
        else:
            self.update_status_disconnected()
    
    def update_status_connected(self):
        """Update status label for connected state"""
        if self.current_theme == "dark":
            self.status_label.setStyleSheet(
                "padding: 10px; background: #1e4620; color: #4ade80; "
                "font-size: 12pt; border-radius: 5px; border: 1px solid #4ade80;"
            )
        else:
            self.status_label.setStyleSheet(
                "padding: 10px; background: #d4edda; color: #155724; "
                "font-size: 12pt; border-radius: 5px; border: 1px solid #c3e6cb;"
            )
    
    def update_status_disconnected(self):
        """Update status label for disconnected state"""
        if self.current_theme == "dark":
            self.status_label.setStyleSheet(
                "padding: 10px; background: #4a1a1a; color: #f87171; "
                "font-size: 12pt; border-radius: 5px; border: 1px solid #f87171;"
            )
        else:
            self.status_label.setStyleSheet(
                "padding: 10px; background: #f8d7da; color: #721c24; "
                "font-size: 12pt; border-radius: 5px; border: 1px solid #f5c6cb;"
            )
        
    def check_connection(self):
        try:
            response = requests.get(f"{self.server_url}/health", timeout=5)
            data = response.json()
            
            if data["success"]:
                self.status_label.setText(f"✅ Connected to {self.server_url}")
                self.update_status_connected()
                self.log_response("System", "Connection successful", data)
            else:
                self.status_label.setText("❌ Server error")
                self.update_status_disconnected()
                
        except Exception as e:
            self.status_label.setText(f"❌ Cannot connect to {self.server_url}")
            self.update_status_disconnected()
            self.log_response("Error", f"Connection failed: {str(e)}")
    
    def log_response(self, action, message, data=None):
        log_entry = f"\n{'='*80}\n[{action.upper()}] {message}\n"
        if data:
            log_entry += f"\n{json.dumps(data, indent=2)}\n"
        self.response_display.append(log_entry)
    
    def create_project(self):
        project_name = self.project_name_input.text().strip()
        
        if not project_name:
            QMessageBox.warning(self, "Input Error", "Project name cannot be empty")
            return
        
        try:
            response = requests.post(
                f"{self.server_url}/api/projects",
                json={"name": project_name, "owner": self.current_user},
                timeout=5
            )
            
            data = response.json()
            
            if data["success"]:
                self.current_project_id = data["data"]["id"]
                self.status_label.setText(f"✅ Current project: {project_name}")
                self.update_status_connected()
                self.log_response("Create Project", f"Project '{project_name}' created successfully", data)
                QMessageBox.information(self, "Success", f"Project '{project_name}' created!")
            else:
                self.log_response("Create Project Error", data['message'])
                QMessageBox.warning(self, "Error", data['message'])
                
        except Exception as e:
            self.log_response("Create Project Error", str(e))
            QMessageBox.critical(self, "Error", str(e))
    
    def list_tools(self):
        try:
            response = requests.get(f"{self.server_url}/api/tools/list", timeout=5)
            data = response.json()
            
            if data["success"]:
                tools = data["data"]["tools"]
                self.log_response("List Tools", f"Found {len(tools)} tools: {', '.join(tools)}", data)
                
                tool_list = "\n".join(f"  • {tool}" for tool in tools)
                QMessageBox.information(self, "Available Tools", f"Simulation tools available:\n\n{tool_list}")
            else:
                self.log_response("List Tools Error", data['message'])
                
        except Exception as e:
            self.log_response("List Tools Error", str(e))
            QMessageBox.critical(self, "Error", str(e))
    
    def upload_file(self):
        if not self.current_project_id:
            QMessageBox.warning(self, "No Project", "Create a project first")
            return
        
        file_path, _ = QFileDialog.getOpenFileName(
            self, "Select File to Upload", "", "All Files (*.*)"
        )
        
        if not file_path:
            return
        
        try:
            with open(file_path, 'rb') as f:
                file_data = f.read()
            
            if len(file_data) > 10 * 1024 * 1024:
                QMessageBox.warning(self, "File Too Large", "File must be under 10 MB")
                return
            
            encoded_data = base64.b64encode(file_data).decode('utf-8')
            filename = Path(file_path).name
            
            response = requests.post(
                f"{self.server_url}/api/files/upload",
                json={
                    "project_id": self.current_project_id,
                    "filename": filename,
                    "data": encoded_data
                },
                timeout=30
            )
            
            data = response.json()
            
            if data["success"]:
                self.log_response("Upload File", f"File '{filename}' uploaded ({data['data']['size']:,} bytes)", data)
                QMessageBox.information(self, "Success", f"File '{filename}' uploaded successfully!")
            else:
                self.log_response("Upload File Error", data['message'])
                QMessageBox.warning(self, "Upload Failed", data['message'])
                
        except Exception as e:
            self.log_response("Upload File Error", str(e))
            QMessageBox.critical(self, "Error", str(e))

if __name__ == "__main__":
    app = QApplication(sys.argv)
    
    # Apply dark theme by default
    app.setStyleSheet(qdarktheme.load_stylesheet("dark"))
    
    window = EDAClient()
    window.show()
    sys.exit(app.exec())