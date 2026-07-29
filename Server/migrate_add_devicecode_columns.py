"""Add authorized_ip, authorized_user_agent, authorized_location to DeviceCode.

Usage: python migrate_add_devicecode_columns.py
"""

import os
import sys

from dotenv import load_dotenv

load_dotenv()

from sqlalchemy import MetaData, Table, create_engine, text

DATABASE_URL = os.getenv('DATABASE_URL', 'sqlite:///instance/syncnotes.db')


def migrate():
    engine = create_engine(DATABASE_URL)
    conn = engine.connect()
    meta = MetaData()
    meta.reflect(bind=engine)

    table = Table('device_code', meta, autoload_with=engine)

    for col, col_type in [
        ('authorized_ip', 'VARCHAR(45)'),
        ('authorized_user_agent', 'VARCHAR(512)'),
        ('authorized_location', 'VARCHAR(200)'),
    ]:
        if col not in table.columns:
            conn.execute(text(f'ALTER TABLE device_code ADD COLUMN {col} {col_type} DEFAULT NULL'))
            print(f'  Added column: {col}')
        else:
            print(f'  Already exists: {col}')

    conn.commit()
    conn.close()
    print('Migration complete.')


if __name__ == '__main__':
    migrate()
