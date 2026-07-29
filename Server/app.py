import os
from urllib.parse import urlparse

import pymysql
from flask import Flask
from flask_login import LoginManager
from sqlalchemy import text

from config import Config
from models import db, User

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
