from datetime import datetime
from functools import wraps

from flask import Blueprint, flash, redirect, render_template, request, url_for
from flask_login import current_user, login_required, login_user, logout_user

from models import ApiToken, DeviceCode, NoteFile, User, db

web_bp = Blueprint('web', __name__)


def admin_required(f):
    @wraps(f)
    @login_required
    def decorated(*args, **kwargs):
        if not current_user.is_admin():
            flash('Admin access required', 'error')
            return redirect(url_for('web.dashboard'))
        return f(*args, **kwargs)
    return decorated


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
        login_input = request.form.get('login', '').strip()
        password = request.form.get('password', '')
        user = User.query.filter(
            (User.username == login_input) | (User.email == login_input)
        ).first()
        if user and user.check_password(password):
            if user.is_locked():
                flash('Account is locked. Contact an admin.', 'error')
            else:
                login_user(user)
                next_page = request.args.get('next')
                return redirect(next_page or url_for('web.dashboard'))
        flash('Invalid login or password', 'error')
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


@web_bp.route('/authorize-device/tokens/<int:token_id>/rename', methods=['POST'])
@login_required
def rename_token(token_id):
    token = ApiToken.query.filter_by(id=token_id, user_id=current_user.id).first_or_404()
    name = request.form.get('name', '').strip()
    if name:
        token.name = f'Device: {name}'
        db.session.commit()
        flash('Token renamed.', 'success')
    return redirect(url_for('web.authorize_device'))


@web_bp.route('/authorize-device/tokens/<int:token_id>/delete', methods=['POST'])
@login_required
def delete_token(token_id):
    token = ApiToken.query.filter_by(id=token_id, user_id=current_user.id).first_or_404()
    db.session.delete(token)
    db.session.commit()
    flash('Token revoked. The app will be logged out on next sync.', 'success')
    return redirect(url_for('web.authorize_device'))


@web_bp.route('/oauth/authorize', methods=['GET', 'POST'])
@login_required
def oauth_authorize():
    device_code_str = request.args.get('device_code', '') or request.form.get('device_code', '')
    code = None

    if device_code_str:
        code = DeviceCode.query.filter_by(
            device_code=device_code_str, is_authorized=False
        ).first()
        if code and code.is_expired():
            flash('Authorization request has expired.', 'error')
            code = None

    if request.method == 'POST':
        if code:
            code.user_id = current_user.id
            code.is_authorized = True
            db.session.commit()
            return render_template('oauth_authorize.html', authorized=True)
        flash('Invalid or expired request.', 'error')
        return redirect(url_for('web.oauth_authorize'))

    return render_template('oauth_authorize.html', authorized=False, code=code)


@web_bp.route('/download')
def download():
    return render_template('download.html')


@web_bp.route('/settings', methods=['GET', 'POST'])
@login_required
def settings():
    if request.method == 'POST':
        action = request.form.get('action', '')

        if action == 'username':
            username = request.form.get('username', '').strip()
            if not username:
                flash('Username is required', 'error')
            elif (
                User.query.filter_by(username=username).first()
                and username != current_user.username
            ):
                flash('Username already taken', 'error')
            else:
                current_user.username = username
                db.session.commit()
                flash('Username updated', 'success')

        elif action == 'email':
            email = request.form.get('email', '').strip()
            if not email:
                flash('Email is required', 'error')
            elif (
                User.query.filter_by(email=email).first()
                and email != current_user.email
            ):
                flash('Email already in use', 'error')
            else:
                current_user.email = email
                db.session.commit()
                flash('Email updated', 'success')

        elif action == 'password':
            current_pw = request.form.get('current_password', '')
            new_pw = request.form.get('new_password', '')
            confirm = request.form.get('confirm_password', '')

            if not current_user.check_password(current_pw):
                flash('Current password is incorrect', 'error')
            elif not new_pw:
                flash('New password is required', 'error')
            elif new_pw != confirm:
                flash('Passwords do not match', 'error')
            else:
                current_user.set_password(new_pw)
                db.session.commit()
                flash('Password updated', 'success')

    return render_template('settings.html')


@web_bp.route('/admin')
@admin_required
def admin_panel():
    users = User.query.order_by(User.created_at.desc()).all()
    return render_template('admin.html', users=users)


@web_bp.route('/admin/users/<int:user_id>/delete', methods=['POST'])
@admin_required
def delete_user(user_id):
    if user_id == current_user.id:
        flash('Cannot delete yourself', 'error')
        return redirect(url_for('web.admin_panel'))
    user = User.query.get_or_404(user_id)
    db.session.delete(user)
    db.session.commit()
    flash(f'User {user.username} deleted', 'success')
    return redirect(url_for('web.admin_panel'))


@web_bp.route('/admin/users/<int:user_id>/toggle-admin', methods=['POST'])
@admin_required
def toggle_admin(user_id):
    if user_id == current_user.id:
        flash('Cannot change your own role', 'error')
        return redirect(url_for('web.admin_panel'))
    user = User.query.get_or_404(user_id)
    user.role = 'user' if user.is_admin() else 'admin'
    db.session.commit()
    flash(f'{user.username} is now {"admin" if user.is_admin() else "user"}', 'success')
    return redirect(url_for('web.admin_panel'))


@web_bp.route('/admin/users/<int:user_id>/edit', methods=['POST'])
@admin_required
def edit_user(user_id):
    user = User.query.get_or_404(user_id)

    username = request.form.get('username', '').strip()
    email = request.form.get('email', '').strip()

    if username and username != user.username:
        if User.query.filter_by(username=username).first():
            flash('Username already taken', 'error')
            return redirect(url_for('web.admin_panel'))
        user.username = username

    if email and email != user.email:
        if User.query.filter_by(email=email).first():
            flash('Email already in use', 'error')
            return redirect(url_for('web.admin_panel'))
        user.email = email

    new_pw = request.form.get('password', '')
    if new_pw:
        user.set_password(new_pw)

    db.session.commit()
    flash(f'User {user.username} updated', 'success')
    return redirect(url_for('web.admin_panel'))


@web_bp.route('/admin/users/<int:user_id>/toggle-lock', methods=['POST'])
@admin_required
def toggle_lock(user_id):
    if user_id == current_user.id:
        flash('Cannot lock yourself', 'error')
        return redirect(url_for('web.admin_panel'))
    user = User.query.get_or_404(user_id)
    user.locked = not user.locked
    db.session.commit()
    status = 'locked' if user.locked else 'unlocked'
    flash(f'{user.username} {status}', 'success')
    return redirect(url_for('web.admin_panel'))
