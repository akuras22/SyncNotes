import pymysql
from config import Config

cfg = Config()
conn = pymysql.connect(
    host=cfg.DB_HOST, port=cfg.DB_PORT,
    user=cfg.DB_USER, password=cfg.DB_PASSWORD,
    database=cfg.DB_NAME,
)
with conn.cursor() as cur:
    for col in [
        "ADD COLUMN last_ip VARCHAR(45) DEFAULT NULL",
        "ADD COLUMN last_user_agent VARCHAR(512) DEFAULT NULL",
        "ADD COLUMN last_location VARCHAR(200) DEFAULT NULL",
    ]:
        try:
            cur.execute(f"ALTER TABLE api_token {col}")
            print(f"OK: {col}")
        except pymysql.err.OperationalError as e:
            print(f"Skip (maybe exists): {e}")
    conn.commit()
conn.close()
