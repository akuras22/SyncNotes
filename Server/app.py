import hashlib
import os
from urllib.parse import urlparse

import pymysql
from flask import Flask
from flask_login import LoginManager
from sqlalchemy import text

from config import Config
from models import db, NoteFile, NoteVersion, User

login_manager = LoginManager()


def ensure_database():
    if 'mysql' not in Config.SQLALCHEMY_DATABASE_URI:
        return
    conn = pymysql.connect(
        host=Config.DB_HOST,
        port=int(Config.DB_PORT),
        user=Config.DB_USER,
        password=Config.DB_PASSWORD,
    )
    with conn.cursor() as cursor:
        cursor.execute(
            f"CREATE DATABASE IF NOT EXISTS `{Config.DB_NAME}` "
            f"CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"
        )
    conn.close()


MIGRATIONS = {
    'api_token': [
        'ADD COLUMN last_ip VARCHAR(45) DEFAULT NULL',
        'ADD COLUMN last_user_agent VARCHAR(512) DEFAULT NULL',
        'ADD COLUMN last_location VARCHAR(200) DEFAULT NULL',
    ],
    'device_code': [
        'ADD COLUMN authorized_ip VARCHAR(45) DEFAULT NULL',
        'ADD COLUMN authorized_user_agent VARCHAR(512) DEFAULT NULL',
        'ADD COLUMN authorized_location VARCHAR(200) DEFAULT NULL',
    ],
    'note_file': [
        'ADD COLUMN relpath VARCHAR(1024) NULL',
        'ADD COLUMN is_deleted TINYINT(1) NOT NULL DEFAULT 0',
        'ADD COLUMN current_version_id INTEGER NULL',
    ],
}


def run_migrations():
    for table, stmts in MIGRATIONS.items():
        for stmt in stmts:
            try:
                db.session.execute(
                    text(f'ALTER TABLE {table} {stmt}')
                )
                db.session.commit()
            except Exception:
                db.session.rollback()


def backfill_note_versions():
    """One-time, idempotent: give pre-existing NoteFile rows (from before
    versioning/folders existed) a relpath and a version-1 NoteVersion
    wrapping their existing single stored file, so they don't disappear
    from the dashboard after the schema change. Safe to call every boot -
    only touches rows where relpath is still NULL.
    """
    pending = NoteFile.query.filter(NoteFile.relpath.is_(None)).all()
    for note in pending:
        relpath = f'{note.name}.rnote'
        collision = NoteFile.query.filter(
            NoteFile.user_id == note.user_id,
            NoteFile.relpath == relpath,
            NoteFile.id != note.id,
        ).first()
        if collision:
            relpath = f'{note.name} ({note.id}).rnote'
        note.relpath = relpath

        if note.original_filename:
            upload_dir = os.path.join(Config.UPLOAD_FOLDER, str(note.user_id))
            src_path = os.path.join(upload_dir, note.original_filename)
            file_size = note.file_size or 0
            content_hash = None
            if os.path.exists(src_path):
                file_size = os.path.getsize(src_path)
                sha256 = hashlib.sha256()
                with open(src_path, 'rb') as f:
                    for chunk in iter(lambda: f.read(65536), b''):
                        sha256.update(chunk)
                content_hash = sha256.hexdigest()

            version = NoteVersion(
                note_file_id=note.id,
                version_number=1,
                storage_filename=note.original_filename,
                file_size=file_size,
                content_hash=content_hash,
            )
            db.session.add(version)
            db.session.flush()
            note.current_version_id = version.id

    if pending:
        db.session.commit()


def create_app():
    ensure_database()

    app = Flask(__name__)
    app.config.from_object(Config)

    os.makedirs(app.config['UPLOAD_FOLDER'], exist_ok=True)

    db.init_app(app)
    login_manager.init_app(app)
    login_manager.login_view = 'web.login'

    @login_manager.user_loader
    def load_user(user_id):
        return db.session.get(User, int(user_id))

    from web import web_bp
    from api import api_bp
    app.register_blueprint(web_bp)
    app.register_blueprint(api_bp)

    with app.app_context():
        db.create_all()
        run_migrations()
        backfill_note_versions()
        if User.query.count() == 0:
            username = os.environ.get('ADMIN_USERNAME', 'admin')
            email = os.environ.get('ADMIN_EMAIL', 'admin@localhost')
            password = os.environ.get('ADMIN_PASSWORD', 'admin123')
            admin = User(username=username, email=email, role='admin')
            admin.set_password(password)
            db.session.add(admin)
            db.session.commit()

    return app


if __name__ == '__main__':
    app = create_app()
    app.run(host='0.0.0.0', port=2394, debug=True)
