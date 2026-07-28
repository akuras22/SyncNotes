import os


class Config:
    SECRET_KEY = os.environ.get('SECRET_KEY', 'change-me-to-a-random-secret')
    SQLALCHEMY_DATABASE_URI = os.environ.get(
        'DATABASE_URL', 'sqlite:///syncnotes.db'
    )
    SQLALCHEMY_TRACK_MODIFICATIONS = False
    UPLOAD_FOLDER = os.path.join(
        os.path.dirname(os.path.abspath(__file__)), 'uploads'
    )
    MAX_CONTENT_LENGTH = 200 * 1024 * 1024
    DEVICE_CODE_EXPIRY = 600
