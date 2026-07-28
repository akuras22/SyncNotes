from datetime import datetime

from flask import Blueprint, flash, redirect, render_template, request, url_for
from flask_login import current_user, login_required, login_user, logout_user

from models import ApiToken, DeviceCode, NoteFile, db

web_bp = Blueprint('web', __name__)


@web_bp.route('/')
@login_required
def dashboard():
    notes = (
        NoteFile.query.filter_by(user_id=current_user.id)
        .order_by(NoteFile.updated_at.desc())
        .all()
    )
    return render_template('dashboard.html', notes=notes)


@web_bp.route('/login', methods=['GET', 'POST'])
def login():
    if current_user.is_authenticated:
        return redirect(url_for('web.dashboard'))

    if request.method == 'POST':
        username = request.form.get('username', '').strip()
        password = request.form.get('password', '')
        user = User.query.filter_by(username=username).first()
        if user and user.check_password(password):
            login_user(user)
            next_page = request.args.get('next')
            return redirect(next_page or url_for('web.dashboard'))
        flash('Invalid username or password', 'error')
    return render_template('login.html')


@web_bp.route('/register', methods=['GET', 'POST'])
def register():
    if current_user.is_authenticated:
        return redirect(url_for('web.dashboard'))

    if request.method == 'POST':
        username = request.form.get('username', '').strip()
        email = request.form.get('email', '').strip()
        password = request.form.get('password', '')
        confirm = request.form.get('confirm', '')

        if not username or not email or not password:
            flash('All fields are required', 'error')
        elif password != confirm:
            flash('Passwords do not match', 'error')
        elif User.query.filter_by(username=username).first():
            flash('Username already taken', 'error')
        elif User.query.filter_by(email=email).first():
            flash('Email already registered', 'error')
        else:
            user = User(username=username, email=email)
            user.set_password(password)
            db.session.add(user)
            db.session.commit()
            login_user(user)
            return redirect(url_for('web.dashboard'))

    return render_template('register.html')


@web_bp.route('/logout')
@login_required
def logout():
    logout_user()
    return redirect(url_for('web.login'))


@web_bp.route('/authorize-device', methods=['GET', 'POST'])
@login_required
def authorize_device():
    if request.method == 'POST':
        user_code = request.form.get('user_code', '').strip().upper()
        code = DeviceCode.query.filter_by(
            user_code=user_code, is_authorized=False
        ).first()

        if not code:
            flash('Invalid or expired code', 'error')
        elif code.is_expired():
            flash('Code has expired, request a new one', 'error')
        else:
            code.user_id = current_user.id
            code.is_authorized = True
            db.session.commit()
            flash('Device authorized successfully!', 'success')

    pending = (
        DeviceCode.query.filter_by(is_authorized=False)
        .filter(DeviceCode.expires_at > datetime.utcnow())
        .count()
    )
    tokens = ApiToken.query.filter_by(user_id=current_user.id).all()
    return render_template(
        'authorize_device.html', pending=pending, tokens=tokens
    )


@web_bp.route('/settings/tokens', methods=['GET', 'POST'])
@login_required
def api_tokens():
    if request.method == 'POST':
        name = request.form.get('name', '').strip()
        if name:
            token_str = ApiToken.generate_token()
            token = ApiToken(
                token=token_str, name=name, user_id=current_user.id
            )
            db.session.add(token)
            db.session.commit()
            flash(f'Token created: {token_str}', 'success')
        else:
            flash('Token name is required', 'error')

    tokens = ApiToken.query.filter_by(user_id=current_user.id).all()
    return render_template('api_tokens.html', tokens=tokens)


@web_bp.route('/settings/tokens/<int:token_id>/revoke', methods=['POST'])
@login_required
def revoke_token(token_id):
    token = ApiToken.query.filter_by(
        id=token_id, user_id=current_user.id
    ).first_or_404()
    db.session.delete(token)
    db.session.commit()
    flash('Token revoked', 'success')
    return redirect(url_for('web.api_tokens'))


from models import User
