import hashlib
import os
import subprocess
import uuid
from datetime import datetime, timedelta
from functools import wraps

import requests
from flask import (
    Blueprint,
    abort,
    current_app,
    g,
    jsonify,
    request,
    send_from_directory,
)
from flask_login import current_user, login_required, login_user
from sqlalchemy.exc import IntegrityError
from werkzeug.utils import secure_filename

from models import ApiToken, DeviceCode, NoteFile, NoteVersion, User, db

api_bp = Blueprint('api', __name__, url_prefix='/api')


def resolve_geo(ip: str) -> str | None:
    try:
        resp = requests.get(
            f'http://ip-api.com/json/{ip}?fields=city,country',
            timeout=3,
        )
        if resp.status_code == 200:
            data = resp.json()
            parts = [p for p in [data.get('city'), data.get('country')] if p]
            return ', '.join(parts) if parts else None
    except Exception:
        pass
    return None


def parse_user_agent(ua: str) -> dict:
    ua = ua or ''

    if 'Windows' in ua:
        os_name = 'Windows'
    elif 'Mac OS X' in ua or 'Macintosh' in ua:
        os_name = 'macOS'
    elif 'Android' in ua:
        os_name = 'Android'
    elif 'iPhone' in ua or 'iPad' in ua:
        os_name = 'iOS'
    elif 'Linux' in ua:
        os_name = 'Linux'
    else:
        os_name = 'Unknown OS'

    if 'Edg/' in ua:
        browser = 'Microsoft Edge'
    elif 'OPR/' in ua or 'Opera' in ua:
        browser = 'Opera'
    elif 'Firefox/' in ua:
        browser = 'Firefox'
    elif 'Chrome/' in ua and 'Chromium' not in ua:
        browser = 'Chrome'
    elif 'Safari/' in ua and 'Chrome' not in ua:
        browser = 'Safari'
    elif 'Gecko/' in ua:
        browser = 'Gecko-based browser'
    else:
        browser = 'Unknown browser'

    return {'os': os_name, 'browser': browser}


def get_client_ip() -> str:
    for header in ('CF-Connecting-IP', 'X-Real-IP'):
        ip = request.headers.get(header, '')
        if ip and '.' in ip:
            return ip
    raw = request.headers.get('X-Forwarded-For', request.remote_addr or '')
    ips = [ip.strip() for ip in raw.split(',') if ip.strip()]
    v4 = [ip for ip in ips if '.' in ip]
    return (v4 or ips)[0]



def require_auth(f):
    @wraps(f)
    def decorated(*args, **kwargs):
        auth = request.headers.get('Authorization', '')
        if auth.startswith('Bearer '):
            token_str = auth[7:]
            token = ApiToken.query.filter_by(token=token_str).first()
            if token:
                ip = get_client_ip()
                ua = (request.headers.get('User-Agent', '') or '')[:512]
                token.last_used_at = datetime.utcnow()
                token.last_ip = ip or None
                token.last_user_agent = ua or None
                if ip and not token.last_location:
                    location = resolve_geo(ip)
                    if location:
                        token.last_location = location
                db.session.commit()
                login_user(token.user)
                g.api_token = token
                return f(*args, **kwargs)
            return jsonify({'error': 'invalid_token'}), 401
        if current_user.is_authenticated:
            return f(*args, **kwargs)
        return jsonify({'error': 'authentication_required'}), 401
    return decorated


def user_upload_dir():
    return os.path.join(
        current_app.config['UPLOAD_FOLDER'], str(current_user.id)
    )


def normalize_relpath(relpath):
    if not relpath:
        return None
    relpath = relpath.replace('\\', '/').strip('/')
    if not relpath:
        return None
    parts = relpath.split('/')
    if any(p in ('', '.', '..') for p in parts):
        return None
    return relpath


def hash_file(path):
    sha256 = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(65536), b''):
            sha256.update(chunk)
    return sha256.hexdigest()


def generate_pdf(rnote_path, pdf_path):
    """Best-effort: render a .rnote file to PDF via rnote-cli, if it's
    installed (see Server/Dockerfile). Never raises - a missing binary,
    a conversion failure, or a timeout just means no PDF this time; the
    desktop app's fallback (uploading its own locally-rendered PDF) can
    fill the gap when the server can't do it."""
    try:
        result = subprocess.run(
            [
                'rnote-cli', 'export', 'doc', rnote_path,
                '--output-file', pdf_path,
                '--on-conflict', 'overwrite',
            ],
            capture_output=True,
            timeout=30,
        )
        return result.returncode == 0 and os.path.exists(pdf_path)
    except (OSError, subprocess.SubprocessError):
        return False


# ─── Device Code Auth ───────────────────────────────────────────────────


@api_bp.route('/auth/device', methods=['POST'])
def request_device_code():
    data = request.get_json(silent=True) or {}
    client_name = data.get('client_name', 'Unknown Device')

    ip = get_client_ip()
    print(f'[device] CF-Connecting-IP={request.headers.get("CF-Connecting-IP","?")} X-Real-IP={request.headers.get("X-Real-IP","?")} X-Forwarded-For={request.headers.get("X-Forwarded-For","?")} remote_addr={request.remote_addr} got={ip}')
    ua = (request.headers.get('User-Agent', '') or '')[:512]
    location = resolve_geo(ip) if ip else None

    code = DeviceCode(
        user_code=DeviceCode.generate_user_code(),
        device_code=ApiToken.generate_token(),
        client_name=client_name,
        authorized_ip=ip,
        authorized_user_agent=ua,
        authorized_location=location,
        expires_at=datetime.utcnow()
        + timedelta(seconds=current_app.config['DEVICE_CODE_EXPIRY']),
    )
    db.session.add(code)
    db.session.commit()

    return jsonify({
        'device_code': code.device_code,
        'user_code': code.user_code,
        'verification_uri': request.host_url.rstrip('/') + f'/oauth/authorize?device_code={code.device_code}',
        'interval': 5,
        'expires_in': current_app.config['DEVICE_CODE_EXPIRY'],
    })


@api_bp.route('/auth/device/status', methods=['GET'])
def check_device_status():
    device_code = request.args.get('device_code', '')
    code = DeviceCode.query.filter_by(device_code=device_code).first()

    if not code:
        return jsonify({'error': 'invalid_device_code'}), 400
    if code.is_expired():
        return jsonify({'status': 'expired'}), 400
    if not code.is_authorized:
        return jsonify({'status': 'authorization_pending'})

    token_str = ApiToken.generate_token()
    ip = get_client_ip()
    ua = (request.headers.get('User-Agent', '') or '')[:512]
    location = None
    if ip:
        location = resolve_geo(ip)
    token = ApiToken(
        token=token_str,
        name=f'Device: {code.client_name}',
        user_id=code.user_id,
        last_ip=code.authorized_ip or ip,
        last_user_agent=code.authorized_user_agent or ua,
        last_location=code.authorized_location or location,
        last_used_at=datetime.utcnow(),
    )
    db.session.add(token)
    db.session.delete(code)
    db.session.commit()

    return jsonify({
        'status': 'authorized',
        'access_token': token_str,
        'token_name': token.name,
    })


# ─── Token Management API ─────────────────────────────────────────────


@api_bp.route('/auth/verify', methods=['GET'])
@require_auth
def verify_token():
    return jsonify({'status': 'ok', 'user': current_user.username})


@api_bp.route('/auth/tokens/<int:token_id>', methods=['PATCH'])
@require_auth
def rename_token(token_id):
    token = ApiToken.query.filter_by(id=token_id, user_id=current_user.id).first_or_404()
    data = request.get_json(silent=True) or {}
    new_name = data.get('name', '').strip()
    if not new_name:
        return jsonify({'error': 'name is required'}), 400
    token.name = new_name
    db.session.commit()
    return jsonify({'status': 'ok', 'name': token.name})


@api_bp.route('/auth/tokens/<int:token_id>', methods=['DELETE'])
@require_auth
def delete_token(token_id):
    token = ApiToken.query.filter_by(id=token_id, user_id=current_user.id).first_or_404()
    db.session.delete(token)
    db.session.commit()
    return jsonify({'status': 'deleted'})


@api_bp.route('/auth/token', methods=['DELETE'])
@require_auth
def revoke_own_token():
    """Let a client revoke the exact token it authenticated with (used by
    the desktop app's Disconnect action, so disconnecting locally also
    removes the device from the user's Authorized Apps on the website)."""
    auth = request.headers.get('Authorization', '')
    if not auth.startswith('Bearer '):
        return jsonify({'error': 'bearer_token_required'}), 400
    token_str = auth[7:]
    token = ApiToken.query.filter_by(token=token_str, user_id=current_user.id).first()
    if token:
        db.session.delete(token)
        db.session.commit()
    return jsonify({'status': 'deleted'})


# ─── Notes Sync API ─────────────────────────────────────────────────────


def _soft_delete(note):
    if not note.is_deleted:
        note.is_deleted = True
        note.updated_at = datetime.utcnow()
        db.session.commit()


@api_bp.route('/notes/manifest', methods=['GET'])
@require_auth
def notes_manifest():
    notes = NoteFile.query.filter_by(user_id=current_user.id).all()
    return jsonify({
        'files': [
            {
                'uuid': n.uuid,
                'relpath': n.relpath,
                'version': n.current_version.version_number if n.current_version else 0,
                'hash': n.current_version.content_hash if n.current_version else None,
                'size': n.file_size,
                'updated_at': n.updated_at.isoformat(),
                'deleted': n.is_deleted,
                'has_pdf': bool(n.current_version and n.current_version.pdf_filename),
            }
            for n in notes
        ]
    })


@api_bp.route('/notes/sync', methods=['POST'])
@require_auth
def sync_note():
    relpath = normalize_relpath(request.form.get('relpath', ''))
    if not relpath or not relpath.lower().endswith('.rnote'):
        return jsonify({'error': 'invalid relpath'}), 400

    uploaded = request.files.get('file')
    if not uploaded:
        return jsonify({'error': 'file is required'}), 400

    upload_dir = user_upload_dir()
    os.makedirs(upload_dir, exist_ok=True)

    tmp_path = os.path.join(upload_dir, f'{uuid.uuid4()}.tmp')
    uploaded.save(tmp_path)
    file_size = os.path.getsize(tmp_path)
    content_hash = hash_file(tmp_path)

    existing = NoteFile.query.filter_by(user_id=current_user.id, relpath=relpath).first()
    if (
        existing
        and not existing.is_deleted
        and existing.current_version
        and existing.current_version.content_hash == content_hash
    ):
        os.remove(tmp_path)
        return jsonify({
            'uuid': existing.uuid,
            'relpath': existing.relpath,
            'version': existing.current_version.version_number,
            'hash': content_hash,
            'has_pdf': bool(existing.current_version.pdf_filename),
        })

    storage_filename = f'{uuid.uuid4()}.rnote'
    os.replace(tmp_path, os.path.join(upload_dir, storage_filename))

    token = getattr(g, 'api_token', None)
    device_name = token.name if token else None
    display_name = os.path.splitext(os.path.basename(relpath))[0]

    version = None
    note = None
    for attempt in range(3):
        note = NoteFile.query.filter_by(user_id=current_user.id, relpath=relpath).first()
        if note is None:
            note = NoteFile(user_id=current_user.id, relpath=relpath, name=display_name, file_size=0)
            db.session.add(note)
        else:
            note.is_deleted = False
            note.name = display_name

        try:
            db.session.flush()
            max_version = (
                db.session.query(db.func.max(NoteVersion.version_number))
                .filter_by(note_file_id=note.id)
                .scalar()
            ) or 0
            version = NoteVersion(
                note_file_id=note.id,
                version_number=max_version + 1,
                storage_filename=storage_filename,
                file_size=file_size,
                content_hash=content_hash,
                device_name=device_name,
            )
            db.session.add(version)
            db.session.flush()
            note.current_version_id = version.id
            note.file_size = file_size
            note.updated_at = datetime.utcnow()
            db.session.commit()
            break
        except IntegrityError:
            db.session.rollback()
            version = None
            if attempt == 2:
                raise

    rnote_full_path = os.path.join(upload_dir, storage_filename)
    pdf_full_path = os.path.join(upload_dir, f'{uuid.uuid4()}.pdf')
    if generate_pdf(rnote_full_path, pdf_full_path):
        version.pdf_filename = os.path.basename(pdf_full_path)
        db.session.commit()

    return jsonify({
        'uuid': note.uuid,
        'relpath': note.relpath,
        'version': version.version_number,
        'hash': content_hash,
        'has_pdf': bool(version.pdf_filename),
    }), 201


@api_bp.route('/notes/delete', methods=['POST'])
@require_auth
def delete_note_sync():
    relpath = normalize_relpath(request.form.get('relpath', ''))
    if not relpath:
        return jsonify({'error': 'invalid relpath'}), 400
    note = NoteFile.query.filter_by(user_id=current_user.id, relpath=relpath).first()
    if note:
        _soft_delete(note)
    return jsonify({'status': 'deleted'})


@api_bp.route('/notes/rename', methods=['POST'])
@require_auth
def rename_note_sync():
    """Renaming/moving a file locally should keep its version history
    instead of looking like a delete-and-recreate, so this updates the
    existing NoteFile's relpath in place. relpath has a unique constraint
    per user (deleted rows keep theirs forever, for history), so a target
    already in use - even by a soft-deleted note - is rejected; the client
    falls back to its normal delete+upload path in that rare case."""
    from_relpath = normalize_relpath(request.form.get('from_relpath', ''))
    to_relpath = normalize_relpath(request.form.get('to_relpath', ''))
    if not from_relpath or not to_relpath or not to_relpath.lower().endswith('.rnote'):
        return jsonify({'error': 'invalid relpath'}), 400

    note = NoteFile.query.filter_by(
        user_id=current_user.id, relpath=from_relpath, is_deleted=False
    ).first()
    if not note:
        return jsonify({'error': 'not found'}), 404

    if from_relpath == to_relpath:
        return jsonify({
            'uuid': note.uuid,
            'relpath': note.relpath,
            'version': note.current_version.version_number if note.current_version else 0,
            'hash': note.current_version.content_hash if note.current_version else None,
            'has_pdf': bool(note.current_version and note.current_version.pdf_filename),
        })

    collision = NoteFile.query.filter_by(user_id=current_user.id, relpath=to_relpath).first()
    if collision and collision.id != note.id:
        return jsonify({'error': 'target already exists'}), 409

    note.relpath = to_relpath
    note.name = os.path.splitext(os.path.basename(to_relpath))[0]
    note.updated_at = datetime.utcnow()
    db.session.commit()

    return jsonify({
        'uuid': note.uuid,
        'relpath': note.relpath,
        'version': note.current_version.version_number if note.current_version else 0,
        'hash': note.current_version.content_hash if note.current_version else None,
        'has_pdf': bool(note.current_version and note.current_version.pdf_filename),
    })


@api_bp.route('/notes/download', methods=['GET'])
@require_auth
def download_note():
    relpath = normalize_relpath(request.args.get('relpath', ''))
    if not relpath:
        abort(404)
    note = NoteFile.query.filter_by(
        user_id=current_user.id, relpath=relpath, is_deleted=False
    ).first()
    if not note:
        abort(404)

    version_number = request.args.get('version', type=int)
    version = (
        NoteVersion.query.filter_by(note_file_id=note.id, version_number=version_number).first()
        if version_number
        else note.current_version
    )
    if not version:
        abort(404)

    return send_from_directory(
        user_upload_dir(),
        version.storage_filename,
        download_name=secure_filename(note.name) + '.rnote',
        as_attachment=True,
    )


@api_bp.route('/notes/pdf', methods=['GET'])
@require_auth
def download_pdf():
    relpath = normalize_relpath(request.args.get('relpath', ''))
    if not relpath:
        abort(404)
    note = NoteFile.query.filter_by(
        user_id=current_user.id, relpath=relpath, is_deleted=False
    ).first()
    if not note:
        abort(404)

    version_number = request.args.get('version', type=int)
    version = (
        NoteVersion.query.filter_by(note_file_id=note.id, version_number=version_number).first()
        if version_number
        else note.current_version
    )
    if not version or not version.pdf_filename:
        abort(404)

    as_attachment = request.args.get('attachment', '') in ('1', 'true')
    return send_from_directory(
        user_upload_dir(),
        version.pdf_filename,
        mimetype='application/pdf',
        as_attachment=as_attachment,
        download_name=(secure_filename(note.name) + '.pdf') if as_attachment else None,
    )


def _admin_note_or_404(note_id):
    if not current_user.is_admin():
        abort(403)
    return NoteFile.query.filter_by(id=note_id, is_deleted=False).first_or_404()


def _admin_version_or_404(note, version_number):
    return (
        NoteVersion.query.filter_by(note_file_id=note.id, version_number=version_number).first()
        if version_number
        else note.current_version
    )


@api_bp.route('/admin/notes/<int:note_id>/download', methods=['GET'])
@login_required
def admin_download_note(note_id):
    note = _admin_note_or_404(note_id)
    version = _admin_version_or_404(note, request.args.get('version', type=int))
    if not version:
        abort(404)
    upload_dir = os.path.join(current_app.config['UPLOAD_FOLDER'], str(note.user_id))
    return send_from_directory(
        upload_dir,
        version.storage_filename,
        download_name=secure_filename(note.name) + '.rnote',
        as_attachment=True,
    )


@api_bp.route('/admin/notes/<int:note_id>/pdf', methods=['GET'])
@login_required
def admin_download_pdf(note_id):
    note = _admin_note_or_404(note_id)
    version = _admin_version_or_404(note, request.args.get('version', type=int))
    if not version or not version.pdf_filename:
        abort(404)
    upload_dir = os.path.join(current_app.config['UPLOAD_FOLDER'], str(note.user_id))
    as_attachment = request.args.get('attachment', '') in ('1', 'true')
    return send_from_directory(
        upload_dir,
        version.pdf_filename,
        mimetype='application/pdf',
        as_attachment=as_attachment,
        download_name=(secure_filename(note.name) + '.pdf') if as_attachment else None,
    )


@api_bp.route('/notes/pdf', methods=['POST'])
@require_auth
def upload_pdf():
    """Device-side fallback: if the server couldn't render a PDF itself
    (rnote-cli not installed on this deployment), a client that has its
    own local Rnote/rnote-cli can generate one and upload it here."""
    relpath = normalize_relpath(request.form.get('relpath', ''))
    version_number = request.form.get('version', type=int)
    if not relpath or not version_number:
        return jsonify({'error': 'relpath and version are required'}), 400

    uploaded = request.files.get('pdf')
    if not uploaded:
        return jsonify({'error': 'pdf file is required'}), 400

    note = NoteFile.query.filter_by(user_id=current_user.id, relpath=relpath).first()
    if not note:
        return jsonify({'error': 'not found'}), 404
    version = NoteVersion.query.filter_by(
        note_file_id=note.id, version_number=version_number
    ).first()
    if not version:
        return jsonify({'error': 'version not found'}), 404

    upload_dir = user_upload_dir()
    os.makedirs(upload_dir, exist_ok=True)
    pdf_filename = version.pdf_filename or f'{uuid.uuid4()}.pdf'
    uploaded.save(os.path.join(upload_dir, pdf_filename))
    version.pdf_filename = pdf_filename
    db.session.commit()

    return jsonify({'status': 'ok'})


@api_bp.route('/notes/<int:note_id>/pdf', methods=['GET'])
@require_auth
def view_pdf(note_id):
    note = NoteFile.query.filter_by(
        id=note_id, user_id=current_user.id
    ).first_or_404()
    if not note.pdf_filename:
        abort(404)
    return send_from_directory(
        user_upload_dir(),
        note.pdf_filename,
        mimetype='application/pdf',
    )


@api_bp.route('/notes/<int:note_id>', methods=['DELETE'])
@require_auth
def delete_note(note_id):
    """JSON API for programmatic clients. The website's own Delete button
    posts to web.delete_note instead (a plain HTML <form> can't send a real
    DELETE, and that route redirects back into the dashboard afterwards
    rather than leaving the browser on this raw JSON response)."""
    note = NoteFile.query.filter_by(
        id=note_id, user_id=current_user.id
    ).first_or_404()
    _soft_delete(note)
    return jsonify({'status': 'deleted'})
