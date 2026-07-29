import os
from datetime import datetime, timedelta
from functools import wraps

import requests
from flask import (
    Blueprint,
    abort,
    current_app,
    jsonify,
    request,
    send_file,
    send_from_directory,
)
from flask_login import current_user, login_user
from werkzeug.utils import secure_filename

from models import ApiToken, DeviceCode, NoteFile, User, db

api_bp = Blueprint('api', __name__, url_prefix='/api')

ALLOWED_EXTENSIONS = {'rnote', 'pdf'}


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


def allowed_file(filename):
    return (
        '.' in filename
        and filename.rsplit('.', 1)[1].lower() in ALLOWED_EXTENSIONS
    )


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


# ─── Notes API ──────────────────────────────────────────────────────────


@api_bp.route('/notes', methods=['GET'])
@require_auth
def list_notes():
    notes = (
        NoteFile.query.filter_by(user_id=current_user.id)
        .order_by(NoteFile.updated_at.desc())
        .all()
    )
    return jsonify([
        {
            'id': n.id,
            'uuid': n.uuid,
            'name': n.name,
            'file_size': n.file_size,
            'has_original': n.original_filename is not None,
            'has_pdf': n.pdf_filename is not None,
            'created_at': n.created_at.isoformat(),
            'updated_at': n.updated_at.isoformat(),
            'download_url': url_for(
                'api.download_original', note_id=n.id, _external=True
            ),
            'pdf_url': url_for(
                'api.view_pdf', note_id=n.id, _external=True
            ) if n.pdf_filename else None,
        }
        for n in notes
    ])


@api_bp.route('/notes/<int:note_id>', methods=['GET'])
@require_auth
def get_note(note_id):
    note = NoteFile.query.filter_by(
        id=note_id, user_id=current_user.id
    ).first_or_404()
    return jsonify({
        'id': note.id,
        'uuid': note.uuid,
        'name': note.name,
        'file_size': note.file_size,
        'has_original': note.original_filename is not None,
        'has_pdf': note.pdf_filename is not None,
        'created_at': note.created_at.isoformat(),
        'updated_at': note.updated_at.isoformat(),
        'download_url': url_for(
            'api.download_original', note_id=note.id, _external=True
        ),
        'pdf_url': url_for(
            'api.view_pdf', note_id=note.id, _external=True
        ) if note.pdf_filename else None,
    })


@api_bp.route('/notes', methods=['POST'])
@require_auth
def upload_note():
    name = request.form.get('name', '').strip()
    original = request.files.get('original')
    pdf = request.files.get('pdf')

    if not original and not pdf:
        return jsonify({'error': 'at least one file required'}), 400
    if not name:
        name = (original or pdf).filename or 'Untitled'

    note = NoteFile(
        name=name, user_id=current_user.id, file_size=0
    )
    db.session.add(note)
    db.session.flush()

    upload_dir = user_upload_dir()
    os.makedirs(upload_dir, exist_ok=True)

    size = 0
    if original and allowed_file(original.filename):
        ext = 'rnote'
        orig_name = f'{note.uuid}.{ext}'
        original.save(os.path.join(upload_dir, orig_name))
        note.original_filename = orig_name
        size += os.path.getsize(os.path.join(upload_dir, orig_name))

    if pdf and allowed_file(pdf.filename):
        pdf_name = f'{note.uuid}.pdf'
        pdf.save(os.path.join(upload_dir, pdf_name))
        note.pdf_filename = pdf_name
        size += os.path.getsize(os.path.join(upload_dir, pdf_name))

    note.file_size = size
    db.session.commit()

    return jsonify({'id': note.id, 'uuid': note.uuid, 'name': note.name}), 201


@api_bp.route('/notes/<int:note_id>', methods=['PUT'])
@require_auth
def update_note(note_id):
    note = NoteFile.query.filter_by(
        id=note_id, user_id=current_user.id
    ).first_or_404()

    name = request.form.get('name', '').strip()
    if name:
        note.name = name

    original = request.files.get('original')
    pdf = request.files.get('pdf')
    upload_dir = user_upload_dir()
    size = 0

    if original and allowed_file(original.filename):
        if note.original_filename:
            old_path = os.path.join(upload_dir, note.original_filename)
            if os.path.exists(old_path):
                os.remove(old_path)
        orig_name = f'{note.uuid}.rnote'
        original.save(os.path.join(upload_dir, orig_name))
        note.original_filename = orig_name

    if pdf and allowed_file(pdf.filename):
        if note.pdf_filename:
            old_path = os.path.join(upload_dir, note.pdf_filename)
            if os.path.exists(old_path):
                os.remove(old_path)
        pdf_name = f'{note.uuid}.pdf'
        pdf.save(os.path.join(upload_dir, pdf_name))
        note.pdf_filename = pdf_name

    if note.original_filename:
        p = os.path.join(upload_dir, note.original_filename)
        if os.path.exists(p):
            size += os.path.getsize(p)
    if note.pdf_filename:
        p = os.path.join(upload_dir, note.pdf_filename)
        if os.path.exists(p):
            size += os.path.getsize(p)

    note.file_size = size
    db.session.commit()

    return jsonify({'id': note.id, 'uuid': note.uuid, 'name': note.name})


@api_bp.route('/notes/<int:note_id>/download', methods=['GET'])
@require_auth
def download_original(note_id):
    note = NoteFile.query.filter_by(
        id=note_id, user_id=current_user.id
    ).first_or_404()
    if not note.original_filename:
        abort(404)
    return send_from_directory(
        user_upload_dir(),
        note.original_filename,
        download_name=secure_filename(note.name) + '.rnote',
        as_attachment=True,
    )


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
    note = NoteFile.query.filter_by(
        id=note_id, user_id=current_user.id
    ).first_or_404()

    upload_dir = user_upload_dir()
    for fname in [note.original_filename, note.pdf_filename]:
        if fname:
            path = os.path.join(upload_dir, fname)
            if os.path.exists(path):
                os.remove(path)

    db.session.delete(note)
    db.session.commit()
    return jsonify({'status': 'deleted'})


from flask import url_for
