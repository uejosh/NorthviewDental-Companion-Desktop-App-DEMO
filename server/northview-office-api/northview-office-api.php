<?php
/**
 * Plugin Name: Northview Office API
 * Description: Private REST API for the Northview Dental Office desktop app (booking requests, patient forms, Smile Savings Plan, review requests, audit log).
 * Version:     1.0.0
 * Author:      Sturl Digital Services
 * Requires PHP: 8.0
 * License:     Proprietary
 *
 * Security model
 *  - Every route needs the device key (X-NVO-Device header). The key is generated here
 *    and entered once into each office PC, so the PIN endpoints are not open to the internet.
 *  - Staff sign in with a 4–6 digit PIN (hashed with wp_hash_password). Five wrong PINs lock
 *    that staff member out for 15 minutes.
 *  - Sessions are random bearer tokens, stored hashed, expiring after 30 minutes idle (12 h max).
 *  - Every change is written to the nvo_audit table against the signed-in staff member.
 */

defined( 'ABSPATH' ) || exit;

final class NVO_Office_API {

	const VERSION    = '1.0.0';
	const DB_VERSION = '1';
	const NS         = 'nvoffice/v1';
	const OPT        = 'nvo_settings';
	const IDLE_TTL   = 1800;
	const MAX_TTL    = 43200;

	const PLANS = array(
		'single'  => array( 'label' => 'Single', 'size' => 1, 'price' => 500 ),
		'dual'    => array( 'label' => 'Dual', 'size' => 2, 'price' => 925 ),
		'family3' => array( 'label' => 'Family of 3', 'size' => 3, 'price' => 1350 ),
		'family4' => array( 'label' => 'Family of 4', 'size' => 4, 'price' => 1500 ),
	);
	const EXTRA_MEMBER = 450;

	/** @var WP_User|null */
	private static $staff = null;

	public static function boot() {
		register_activation_hook( __FILE__, array( __CLASS__, 'install' ) );
		add_action( 'plugins_loaded', array( __CLASS__, 'maybe_upgrade' ) );
		add_action( 'rest_api_init', array( __CLASS__, 'routes' ) );
		add_action( 'admin_menu', array( __CLASS__, 'admin_menu' ) );
		add_action( 'admin_post_nvo_save', array( __CLASS__, 'admin_save' ) );
		add_action( 'wpforms_process_complete', array( __CLASS__, 'on_entry' ), 10, 4 );
	}

	/* ------------------------------------------------------------------ setup */

	private static function t( $name ) {
		global $wpdb;
		return $wpdb->prefix . 'nvo_' . $name;
	}

	public static function install() {
		global $wpdb;
		require_once ABSPATH . 'wp-admin/includes/upgrade.php';
		$c = $wpdb->get_charset_collate();
		dbDelta( "CREATE TABLE " . self::t( 'request_state' ) . " (
			entry_id BIGINT UNSIGNED NOT NULL,
			status VARCHAR(20) NOT NULL DEFAULT 'new',
			first_action_at DATETIME NULL,
			appt_date DATE NULL,
			updated_at DATETIME NOT NULL,
			PRIMARY KEY  (entry_id)
		) $c;" );
		dbDelta( "CREATE TABLE " . self::t( 'form_state' ) . " (
			entry_id BIGINT UNSIGNED NOT NULL,
			attached_at DATETIME NULL,
			attached_by VARCHAR(100) NOT NULL DEFAULT '',
			PRIMARY KEY  (entry_id)
		) $c;" );
		dbDelta( "CREATE TABLE " . self::t( 'audit' ) . " (
			id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
			object_type VARCHAR(20) NOT NULL,
			object_id BIGINT UNSIGNED NOT NULL DEFAULT 0,
			action VARCHAR(60) NOT NULL,
			detail TEXT NULL,
			user_id BIGINT UNSIGNED NOT NULL DEFAULT 0,
			user_name VARCHAR(100) NOT NULL DEFAULT '',
			ip VARCHAR(64) NOT NULL DEFAULT '',
			created_at DATETIME NOT NULL,
			PRIMARY KEY  (id),
			KEY obj (object_type, object_id)
		) $c;" );
		dbDelta( "CREATE TABLE " . self::t( 'members' ) . " (
			id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
			name VARCHAR(190) NOT NULL,
			plan VARCHAR(20) NOT NULL,
			household TEXT NOT NULL,
			enrolled DATE NOT NULL,
			renews DATE NOT NULL,
			payment VARCHAR(10) NOT NULL DEFAULT 'due',
			last_reminder DATETIME NULL,
			archived TINYINT(1) NOT NULL DEFAULT 0,
			created_at DATETIME NOT NULL,
			updated_at DATETIME NOT NULL,
			PRIMARY KEY  (id)
		) $c;" );
		dbDelta( "CREATE TABLE " . self::t( 'visits' ) . " (
			id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
			name VARCHAR(190) NOT NULL,
			phone VARCHAR(40) NOT NULL DEFAULT '',
			email VARCHAR(190) NOT NULL DEFAULT '',
			visit_date DATE NOT NULL,
			request_id BIGINT UNSIGNED NOT NULL DEFAULT 0,
			created_at DATETIME NOT NULL,
			PRIMARY KEY  (id),
			KEY request_id (request_id)
		) $c;" );
		dbDelta( "CREATE TABLE " . self::t( 'review_log' ) . " (
			id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
			visit_id BIGINT UNSIGNED NOT NULL,
			name VARCHAR(190) NOT NULL,
			phone VARCHAR(40) NOT NULL DEFAULT '',
			email VARCHAR(190) NOT NULL DEFAULT '',
			channel VARCHAR(10) NOT NULL,
			user_name VARCHAR(100) NOT NULL DEFAULT '',
			created_at DATETIME NOT NULL,
			PRIMARY KEY  (id)
		) $c;" );
		dbDelta( "CREATE TABLE " . self::t( 'sessions' ) . " (
			token_hash CHAR(64) NOT NULL,
			user_id BIGINT UNSIGNED NOT NULL,
			created_at DATETIME NOT NULL,
			last_seen DATETIME NOT NULL,
			PRIMARY KEY  (token_hash)
		) $c;" );

		$s = self::settings();
		if ( empty( $s['device_key'] ) ) {
			$s['device_key'] = self::new_device_key();
		}
		update_option( self::OPT, $s, false );
		update_option( 'nvo_db_version', self::DB_VERSION, false );
	}

	public static function maybe_upgrade() {
		if ( get_option( 'nvo_db_version' ) !== self::DB_VERSION ) {
			self::install();
		}
	}

	private static function new_device_key() {
		return 'nvo_' . bin2hex( random_bytes( 24 ) );
	}

	public static function settings() {
		$d = array(
			'device_key'         => '',
			'booking_form_id'    => 55,
			'patient_form_id'    => 0,
			'patient_form_url'   => '',
			'review_link'        => '',
			'google_profile_url' => '',
			'google_rating'      => '5.0',
			'practice_phone'     => '317.818.7777',
			'messaging_live'     => 0,
		);
		return wp_parse_args( (array) get_option( self::OPT, array() ), $d );
	}

	/* ------------------------------------------------------------------ helpers */

	private static function now() {
		return gmdate( 'Y-m-d H:i:s' );
	}

	private static function iso( $mysql_gmt ) {
		if ( empty( $mysql_gmt ) || '0000-00-00 00:00:00' === $mysql_gmt ) {
			return null;
		}
		return gmdate( 'Y-m-d\TH:i:s\Z', strtotime( $mysql_gmt . ' UTC' ) );
	}

	private static function today() {
		return wp_date( 'Y-m-d' );
	}

	private static function ip() {
		return isset( $_SERVER['REMOTE_ADDR'] ) ? sanitize_text_field( wp_unslash( $_SERVER['REMOTE_ADDR'] ) ) : '';
	}

	private static function staff_name( WP_User $u ) {
		$label = get_user_meta( $u->ID, 'nvo_label', true );
		return $label ? $label : $u->display_name;
	}

	private static function audit( $type, $id, $action, $detail = '' ) {
		global $wpdb;
		$u = self::$staff;
		$wpdb->insert(
			self::t( 'audit' ),
			array(
				'object_type' => $type,
				'object_id'   => (int) $id,
				'action'      => $action,
				'detail'      => $detail,
				'user_id'     => $u ? $u->ID : 0,
				'user_name'   => $u ? self::staff_name( $u ) : 'Website',
				'ip'          => self::ip(),
				'created_at'  => self::now(),
			)
		);
	}

	private static function err( $code, $msg, $status ) {
		return new WP_Error( $code, $msg, array( 'status' => $status ) );
	}

	/* ------------------------------------------------------------------ auth */

	private static function device_ok( WP_REST_Request $r ) {
		$key = (string) $r->get_header( 'x_nvo_device' );
		$s   = self::settings();
		return '' !== $key && '' !== $s['device_key'] && hash_equals( $s['device_key'], $key );
	}

	public static function perm_device( WP_REST_Request $r ) {
		return self::device_ok( $r ) ? true : self::err( 'nvo_device', 'This computer is not registered with the website.', 401 );
	}

	public static function perm_staff( WP_REST_Request $r ) {
		global $wpdb;
		if ( ! self::device_ok( $r ) ) {
			return self::err( 'nvo_device', 'This computer is not registered with the website.', 401 );
		}
		$auth = (string) $r->get_header( 'authorization' );
		if ( ! preg_match( '/^Bearer\s+([a-f0-9]{64})$/i', $auth, $m ) ) {
			return self::err( 'nvo_session', 'Please sign in again.', 401 );
		}
		$hash = hash( 'sha256', strtolower( $m[1] ) );
		$row  = $wpdb->get_row( $wpdb->prepare( 'SELECT * FROM ' . self::t( 'sessions' ) . ' WHERE token_hash = %s', $hash ) );
		$now  = time();
		if ( ! $row
			|| strtotime( $row->last_seen . ' UTC' ) < $now - self::IDLE_TTL
			|| strtotime( $row->created_at . ' UTC' ) < $now - self::MAX_TTL ) {
			if ( $row ) {
				$wpdb->delete( self::t( 'sessions' ), array( 'token_hash' => $hash ) );
			}
			return self::err( 'nvo_session', 'Your session has ended. Please sign in again.', 401 );
		}
		$u = get_user_by( 'id', (int) $row->user_id );
		if ( ! $u || ! get_user_meta( $u->ID, 'nvo_enabled', true ) ) {
			return self::err( 'nvo_session', 'This staff account is no longer active.', 403 );
		}
		$wpdb->update( self::t( 'sessions' ), array( 'last_seen' => self::now() ), array( 'token_hash' => $hash ) );
		self::$staff = $u;
		return true;
	}

	private static function staff_users() {
		return get_users(
			array(
				'meta_key'   => 'nvo_enabled',
				'meta_value' => '1',
				'orderby'    => 'display_name',
			)
		);
	}

	private static function staff_out( WP_User $u ) {
		$name  = self::staff_name( $u );
		$parts = preg_split( '/\s+/', trim( $name ) );
		$ini   = strtoupper( substr( $parts[0], 0, 1 ) . ( isset( $parts[1] ) ? substr( $parts[1], 0, 1 ) : substr( $parts[0], 1, 1 ) ) );
		return array(
			'id'       => $u->ID,
			'name'     => $name,
			'role'     => get_user_meta( $u->ID, 'nvo_role_label', true ) ?: 'Front desk',
			'initials' => $ini,
			'manager'  => (bool) get_user_meta( $u->ID, 'nvo_manager', true ),
		);
	}

	public static function r_staff() {
		$out = array();
		foreach ( self::staff_users() as $u ) {
			if ( get_user_meta( $u->ID, 'nvo_pin_hash', true ) ) {
				$out[] = self::staff_out( $u );
			}
		}
		return array( 'staff' => $out, 'site' => get_bloginfo( 'name' ), 'host' => wp_parse_url( home_url(), PHP_URL_HOST ) );
	}

	public static function r_login( WP_REST_Request $r ) {
		global $wpdb;
		$uid = (int) $r->get_param( 'user_id' );
		$pin = (string) $r->get_param( 'pin' );
		$u   = get_user_by( 'id', $uid );
		if ( ! $u || ! get_user_meta( $uid, 'nvo_enabled', true ) ) {
			return self::err( 'nvo_login', 'Unknown staff member.', 403 );
		}
		$lock_key = 'nvo_lock_' . $uid;
		$fails    = (int) get_transient( $lock_key );
		if ( $fails >= 5 ) {
			return self::err( 'nvo_locked', 'Too many wrong PINs. Try again in 15 minutes or ask the office manager.', 429 );
		}
		$hash = get_user_meta( $uid, 'nvo_pin_hash', true );
		if ( ! preg_match( '/^\d{4,6}$/', $pin ) || ! $hash || ! wp_check_password( $pin, $hash, $uid ) ) {
			set_transient( $lock_key, $fails + 1, 15 * MINUTE_IN_SECONDS );
			self::$staff = $u;
			self::audit( 'session', 0, 'pin_failed' );
			self::$staff = null;
			return self::err( 'nvo_login', 'That PIN is not right.', 403 );
		}
		delete_transient( $lock_key );
		$token = bin2hex( random_bytes( 32 ) );
		$wpdb->query( $wpdb->prepare( 'DELETE FROM ' . self::t( 'sessions' ) . ' WHERE last_seen < %s', gmdate( 'Y-m-d H:i:s', time() - self::MAX_TTL ) ) );
		$wpdb->insert(
			self::t( 'sessions' ),
			array(
				'token_hash' => hash( 'sha256', $token ),
				'user_id'    => $uid,
				'created_at' => self::now(),
				'last_seen'  => self::now(),
			)
		);
		self::$staff = $u;
		self::audit( 'session', 0, 'signed_in' );
		return array( 'token' => $token, 'staff' => self::staff_out( $u ), 'idle_ttl' => self::IDLE_TTL );
	}

	public static function r_logout( WP_REST_Request $r ) {
		global $wpdb;
		if ( preg_match( '/^Bearer\s+([a-f0-9]{64})$/i', (string) $r->get_header( 'authorization' ), $m ) ) {
			$wpdb->delete( self::t( 'sessions' ), array( 'token_hash' => hash( 'sha256', strtolower( $m[1] ) ) ) );
		}
		self::audit( 'session', 0, 'signed_out' );
		return array( 'ok' => true );
	}

	/* ------------------------------------------------------------------ booking requests */

	private static function entry_rows( $form_id, $days = 120 ) {
		global $wpdb;
		if ( ! $form_id ) {
			return array();
		}
		$e = $wpdb->prefix . 'wpforms_entries';
		$s = self::t( 'request_state' );
		return $wpdb->get_results(
			$wpdb->prepare(
				"SELECT e.entry_id, e.fields, e.date, s.status, s.first_action_at, s.appt_date
				 FROM $e e LEFT JOIN $s s ON s.entry_id = e.entry_id
				 WHERE e.form_id = %d AND e.status NOT IN ('trash','spam')
				   AND ( e.date >= %s OR s.status IS NULL OR s.status IN ('new','contacted') )
				 ORDER BY e.date DESC LIMIT 500",
				$form_id,
				gmdate( 'Y-m-d H:i:s', time() - $days * DAY_IN_SECONDS )
			)
		);
	}

	/** Flatten WPForms entry fields into label => value (repeater rows get a numeric suffix list). */
	private static function entry_values( $json ) {
		$fields = json_decode( (string) $json, true );
		$flat   = array();
		$multi  = array();
		if ( ! is_array( $fields ) ) {
			return array( $flat, $multi );
		}
		foreach ( $fields as $key => $f ) {
			$label = strtolower( trim( (string) ( $f['name'] ?? '' ) ) );
			$val   = is_scalar( $f['value'] ?? '' ) ? trim( (string) $f['value'] ) : '';
			if ( '' === $label ) {
				continue;
			}
			$multi[ $label ][] = $val;
			if ( ! isset( $flat[ $label ] ) || '' === $flat[ $label ] ) {
				$flat[ $label ] = $val;
			}
		}
		return array( $flat, $multi );
	}

	private static function pick( array $flat, array $needles ) {
		foreach ( $flat as $label => $v ) {
			foreach ( $needles as $n ) {
				if ( false !== strpos( $label, $n ) && '' !== $v ) {
					return $v;
				}
			}
		}
		return '';
	}

	private static function request_out( $row ) {
		list( $f, $m ) = self::entry_values( $row->fields );
		$days          = $m['preferred day'] ?? array();
		$times         = $m['preferred time'] ?? array();
		$pref          = array();
		foreach ( $days as $i => $d ) {
			if ( '' === $d ) {
				continue;
			}
			$pref[] = trim( $d . ( ! empty( $times[ $i ] ) ? ' · ' . $times[ $i ] : '' ) );
		}
		$addr   = array_filter( array( self::pick( $f, array( 'street' ) ), self::pick( $f, array( 'apt' ) ) ) );
		$city   = trim( implode( ', ', array_filter( array( self::pick( $f, array( 'city' ) ), trim( self::pick( $f, array( 'state' ) ) . ' ' . self::pick( $f, array( 'zip' ) ) ) ) ) ) );
		$reason = self::pick( $f, array( 'reason' ) );
		$status = self::pick( $f, array( 'patient status' ) );
		return array(
			'id'              => (int) $row->entry_id,
			'ref'             => 'R-' . (int) $row->entry_id,
			'first_name'      => self::pick( $f, array( 'first name' ) ),
			'last_name'       => self::pick( $f, array( 'last name' ) ),
			'phone'           => self::pick( $f, array( 'phone' ) ),
			'email'           => self::pick( $f, array( 'email' ) ),
			'address'         => trim( implode( ', ', array_filter( array( implode( ', ', $addr ), $city ) ) ) ),
			'dob'             => self::pick( $f, array( 'date of birth', 'birth' ) ),
			'insurance'       => self::pick( $f, array( 'insurance' ) ),
			'reason'          => $reason,
			'new_patient'     => false === stripos( $status, 'returning' ),
			'emergency'       => false !== stripos( $reason, 'emergency' ),
			'preferred'       => $pref,
			'message'         => self::pick( $f, array( 'message' ) ),
			'received_at'     => self::iso( $row->date ),
			'status'          => $row->status ? $row->status : 'new',
			'first_action_at' => self::iso( $row->first_action_at ),
			'appt_date'       => $row->appt_date,
		);
	}

	private static function activity_for( $type, array $ids ) {
		global $wpdb;
		if ( ! $ids ) {
			return array();
		}
		$in   = implode( ',', array_map( 'intval', $ids ) );
		$rows = $wpdb->get_results( $wpdb->prepare( 'SELECT object_id, action, detail, user_name, created_at FROM ' . self::t( 'audit' ) . " WHERE object_type = %s AND object_id IN ($in) ORDER BY id DESC", $type ) );
		$out  = array();
		foreach ( $rows as $a ) {
			$out[ (int) $a->object_id ][] = array(
				'action' => $a->action,
				'detail' => $a->detail,
				'by'     => $a->user_name,
				'at'     => self::iso( $a->created_at ),
			);
		}
		return $out;
	}

	private static function requests() {
		$s    = self::settings();
		$rows = self::entry_rows( (int) $s['booking_form_id'] );
		$out  = array_map( array( __CLASS__, 'request_out' ), $rows );
		$act  = self::activity_for( 'request', wp_list_pluck( $out, 'id' ) );
		foreach ( $out as &$o ) {
			$o['activity'] = $act[ $o['id'] ] ?? array();
		}
		return $out;
	}

	public static function r_request_status( WP_REST_Request $r ) {
		global $wpdb;
		$id     = (int) $r['id'];
		$status = (string) $r->get_param( 'status' );
		$note   = sanitize_text_field( (string) $r->get_param( 'note' ) );
		$appt   = (string) $r->get_param( 'appt_date' );
		if ( ! in_array( $status, array( 'new', 'contacted', 'scheduled', 'declined' ), true ) ) {
			return self::err( 'nvo_status', 'Unknown status.', 400 );
		}
		if ( $appt && ! preg_match( '/^\d{4}-\d{2}-\d{2}$/', $appt ) ) {
			return self::err( 'nvo_date', 'Appointment date must be YYYY-MM-DD.', 400 );
		}
		$s     = self::settings();
		$entry = $wpdb->get_row( $wpdb->prepare( "SELECT entry_id, fields, date FROM {$wpdb->prefix}wpforms_entries WHERE entry_id = %d AND form_id = %d", $id, (int) $s['booking_form_id'] ) );
		if ( ! $entry ) {
			return self::err( 'nvo_missing', 'That request no longer exists on the website.', 404 );
		}
		$t    = self::t( 'request_state' );
		$prev = $wpdb->get_row( $wpdb->prepare( "SELECT * FROM $t WHERE entry_id = %d", $id ) );
		$data = array(
			'entry_id'        => $id,
			'status'          => $status,
			'first_action_at' => $prev && $prev->first_action_at ? $prev->first_action_at : ( 'new' !== $status ? self::now() : null ),
			'appt_date'       => $appt ? $appt : ( $prev ? $prev->appt_date : null ),
			'updated_at'      => self::now(),
		);
		$wpdb->replace( $t, $data );
		$detail = trim( $note . ( $appt ? ( $note ? ' · ' : '' ) . 'Appointment ' . $appt : '' ) );
		self::audit( 'request', $id, 'status_' . $status, $detail );

		if ( 'scheduled' === $status && $appt ) {
			$req = self::request_out( (object) array_merge( (array) $entry, array( 'status' => $status, 'first_action_at' => null, 'appt_date' => $appt ) ) );
			$vt  = self::t( 'visits' );
			$vid = (int) $wpdb->get_var( $wpdb->prepare( "SELECT id FROM $vt WHERE request_id = %d", $id ) );
			$row = array(
				'name'       => trim( $req['first_name'] . ' ' . $req['last_name'] ),
				'phone'      => $req['phone'],
				'email'      => $req['email'],
				'visit_date' => $appt,
				'request_id' => $id,
			);
			if ( $vid ) {
				$wpdb->update( $vt, $row, array( 'id' => $vid ) );
			} else {
				$wpdb->insert( $vt, $row + array( 'created_at' => self::now() ) );
			}
		}
		return self::snapshot();
	}

	public static function r_request_viewed( WP_REST_Request $r ) {
		self::audit( 'request', (int) $r['id'], 'viewed' );
		return array( 'ok' => true );
	}

	public static function r_request_copied( WP_REST_Request $r ) {
		self::audit( 'request', (int) $r['id'], 'copied' );
		return array( 'ok' => true );
	}

	/** Website hook: write the "received" audit row the moment a booking arrives. */
	public static function on_entry( $fields, $entry, $form_data, $entry_id ) {
		$s = self::settings();
		if ( (int) ( $form_data['id'] ?? 0 ) === (int) $s['booking_form_id'] && $entry_id ) {
			self::audit( 'request', (int) $entry_id, 'received' );
		}
		if ( (int) $s['patient_form_id'] && (int) ( $form_data['id'] ?? 0 ) === (int) $s['patient_form_id'] && $entry_id ) {
			self::audit( 'form', (int) $entry_id, 'received' );
		}
	}

	/* ------------------------------------------------------------------ patient forms */

	private static function patient_forms() {
		global $wpdb;
		$s   = self::settings();
		$fid = (int) $s['patient_form_id'];
		if ( ! $fid ) {
			return array();
		}
		$e    = $wpdb->prefix . 'wpforms_entries';
		$st   = self::t( 'form_state' );
		$rows = $wpdb->get_results( $wpdb->prepare( "SELECT e.entry_id, e.fields, e.date, s.attached_at, s.attached_by FROM $e e LEFT JOIN $st s ON s.entry_id = e.entry_id WHERE e.form_id = %d AND e.status NOT IN ('trash','spam') AND ( s.attached_at IS NULL OR e.date >= %s ) ORDER BY e.date DESC LIMIT 200", $fid, gmdate( 'Y-m-d H:i:s', time() - 60 * DAY_IN_SECONDS ) ) );
		$out  = array();
		foreach ( $rows as $row ) {
			list( $f ) = self::entry_values( $row->fields );
			$name      = self::pick( $f, array( 'full name', 'patient name' ) );
			if ( '' === $name ) {
				$name = trim( self::pick( $f, array( 'first name' ) ) . ' ' . self::pick( $f, array( 'last name' ) ) );
			}
			if ( '' === $name ) {
				$name = self::pick( $f, array( 'name' ) );
			}
			$out[] = array(
				'id'           => (int) $row->entry_id,
				'ref'          => 'F-' . (int) $row->entry_id,
				'name'         => $name,
				'dob'          => self::pick( $f, array( 'date of birth', 'birth' ) ),
				'phone'        => self::pick( $f, array( 'phone' ) ),
				'email'        => self::pick( $f, array( 'email' ) ),
				'address'      => self::pick( $f, array( 'address' ) ),
				'visit_date'   => self::pick( $f, array( 'first visit', 'appointment', 'visit date' ) ),
				'conditions'   => self::pick( $f, array( 'condition', 'medical history' ) ),
				'medications'  => self::pick( $f, array( 'medication' ) ),
				'allergies'    => self::pick( $f, array( 'allerg' ) ),
				'last_dental'  => self::pick( $f, array( 'last dental', 'last visit' ) ),
				'carrier'      => self::pick( $f, array( 'carrier', 'insurance company', 'insurance' ) ),
				'member_id'    => self::pick( $f, array( 'member id', 'subscriber id', 'policy' ) ),
				'signature'    => self::pick( $f, array( 'signature', 'signed' ) ),
				'submitted_at' => self::iso( $row->date ),
				'attached_at'  => self::iso( $row->attached_at ),
				'attached_by'  => (string) $row->attached_by,
			);
		}
		return $out;
	}

	public static function r_form_attached( WP_REST_Request $r ) {
		global $wpdb;
		$id = (int) $r['id'];
		$wpdb->replace(
			self::t( 'form_state' ),
			array(
				'entry_id'    => $id,
				'attached_at' => self::now(),
				'attached_by' => self::staff_name( self::$staff ),
			)
		);
		self::audit( 'form', $id, 'attached' );
		return self::snapshot();
	}

	public static function r_form_saved_pdf( WP_REST_Request $r ) {
		self::audit( 'form', (int) $r['id'], 'saved_pdf' );
		return array( 'ok' => true );
	}

	public static function r_form_link( WP_REST_Request $r ) {
		$rid = (int) $r->get_param( 'request_id' );
		self::audit( 'request', $rid, 'form_link_sent', self::settings()['messaging_live'] ? 'sent' : 'simulated' );
		return self::snapshot();
	}

	/* ------------------------------------------------------------------ savings plan */

	private static function price( $plan, $people ) {
		$p = self::PLANS[ $plan ] ?? self::PLANS['single'];
		return $p['price'] + max( 0, $people - $p['size'] ) * self::EXTRA_MEMBER;
	}

	private static function members() {
		global $wpdb;
		$rows = $wpdb->get_results( 'SELECT * FROM ' . self::t( 'members' ) . ' WHERE archived = 0 ORDER BY renews ASC' );
		$out  = array();
		foreach ( $rows as $m ) {
			$house = json_decode( $m->household, true ) ?: array();
			$out[] = array(
				'id'            => (int) $m->id,
				'name'          => $m->name,
				'plan'          => $m->plan,
				'household'     => $house,
				'enrolled'      => $m->enrolled,
				'renews'        => $m->renews,
				'payment'       => $m->payment,
				'price'         => self::price( $m->plan, count( $house ) ),
				'last_reminder' => self::iso( $m->last_reminder ),
			);
		}
		return $out;
	}

	public static function r_member_add( WP_REST_Request $r ) {
		global $wpdb;
		$name  = sanitize_text_field( (string) $r->get_param( 'name' ) );
		$plan  = (string) $r->get_param( 'plan' );
		$enr   = (string) $r->get_param( 'enrolled' );
		$house = array_values( array_filter( array_map( 'sanitize_text_field', (array) $r->get_param( 'household' ) ) ) );
		if ( '' === $name || ! isset( self::PLANS[ $plan ] ) || ! preg_match( '/^\d{4}-\d{2}-\d{2}$/', $enr ) ) {
			return self::err( 'nvo_member', 'Name, plan and enrolment date are required.', 400 );
		}
		if ( ! $house ) {
			$house = array( $name );
		}
		$wpdb->insert(
			self::t( 'members' ),
			array(
				'name'       => $name,
				'plan'       => $plan,
				'household'  => wp_json_encode( $house ),
				'enrolled'   => $enr,
				'renews'     => gmdate( 'Y-m-d', strtotime( $enr . ' +1 year' ) ),
				'payment'    => 'paid' === $r->get_param( 'payment' ) ? 'paid' : 'due',
				'created_at' => self::now(),
				'updated_at' => self::now(),
			)
		);
		self::audit( 'member', $wpdb->insert_id, 'added', self::PLANS[ $plan ]['label'] );
		return self::snapshot();
	}

	public static function r_member_action( WP_REST_Request $r ) {
		global $wpdb;
		$id = (int) $r['id'];
		$t  = self::t( 'members' );
		$m  = $wpdb->get_row( $wpdb->prepare( "SELECT * FROM $t WHERE id = %d AND archived = 0", $id ) );
		if ( ! $m ) {
			return self::err( 'nvo_missing', 'Membership not found.', 404 );
		}
		switch ( $r['action'] ) {
			case 'paid':
				$wpdb->update( $t, array( 'payment' => 'paid', 'updated_at' => self::now() ), array( 'id' => $id ) );
				break;
			case 'renew':
				$wpdb->update( $t, array( 'renews' => gmdate( 'Y-m-d', strtotime( $m->renews . ' +1 year' ) ), 'payment' => 'due', 'updated_at' => self::now() ), array( 'id' => $id ) );
				break;
			case 'remind':
				$wpdb->update( $t, array( 'last_reminder' => self::now(), 'updated_at' => self::now() ), array( 'id' => $id ) );
				break;
			case 'archive':
				$wpdb->update( $t, array( 'archived' => 1, 'updated_at' => self::now() ), array( 'id' => $id ) );
				break;
			default:
				return self::err( 'nvo_action', 'Unknown action.', 400 );
		}
		self::audit( 'member', $id, $r['action'], 'remind' === $r['action'] && ! self::settings()['messaging_live'] ? 'simulated' : '' );
		return self::snapshot();
	}

	/* ------------------------------------------------------------------ reviews */

	private static function visits() {
		global $wpdb;
		$vt   = self::t( 'visits' );
		$lt   = self::t( 'review_log' );
		$from = wp_date( 'Y-m-d', time() - 7 * DAY_IN_SECONDS );
		$rows = $wpdb->get_results( $wpdb->prepare( "SELECT * FROM $vt WHERE visit_date BETWEEN %s AND %s ORDER BY visit_date DESC, id DESC", $from, self::today() ) );
		$out  = array();
		foreach ( $rows as $v ) {
			// Last asked is matched by phone or email so a patient is never asked twice within the cooldown.
			$last = $wpdb->get_var(
				$wpdb->prepare(
					"SELECT MAX(created_at) FROM $lt WHERE (phone <> '' AND phone = %s) OR (email <> '' AND email = %s) OR visit_id = %d",
					$v->phone,
					$v->email,
					$v->id
				)
			);
			$out[] = array(
				'id'         => (int) $v->id,
				'name'       => $v->name,
				'phone'      => $v->phone,
				'email'      => $v->email,
				'visit_date' => $v->visit_date,
				'last_asked' => self::iso( $last ),
			);
		}
		return $out;
	}

	private static function review_log() {
		global $wpdb;
		$rows = $wpdb->get_results( 'SELECT * FROM ' . self::t( 'review_log' ) . ' ORDER BY id DESC LIMIT 100' );
		return array_map(
			fn( $l ) => array(
				'id'      => (int) $l->id,
				'name'    => $l->name,
				'channel' => $l->channel,
				'by'      => $l->user_name,
				'at'      => self::iso( $l->created_at ),
			),
			$rows
		);
	}

	public static function r_visit_add( WP_REST_Request $r ) {
		global $wpdb;
		$name = sanitize_text_field( (string) $r->get_param( 'name' ) );
		$date = (string) $r->get_param( 'visit_date' );
		if ( '' === $name || ! preg_match( '/^\d{4}-\d{2}-\d{2}$/', $date ) ) {
			return self::err( 'nvo_visit', 'Name and visit date are required.', 400 );
		}
		$wpdb->insert(
			self::t( 'visits' ),
			array(
				'name'       => $name,
				'phone'      => sanitize_text_field( (string) $r->get_param( 'phone' ) ),
				'email'      => sanitize_email( (string) $r->get_param( 'email' ) ),
				'visit_date' => $date,
				'created_at' => self::now(),
			)
		);
		self::audit( 'visit', $wpdb->insert_id, 'added' );
		return self::snapshot();
	}

	public static function r_review_send( WP_REST_Request $r ) {
		global $wpdb;
		$vid     = (int) $r->get_param( 'visit_id' );
		$channel = 'email' === $r->get_param( 'channel' ) ? 'email' : 'text';
		$v       = $wpdb->get_row( $wpdb->prepare( 'SELECT * FROM ' . self::t( 'visits' ) . ' WHERE id = %d', $vid ) );
		if ( ! $v ) {
			return self::err( 'nvo_missing', 'Visit not found.', 404 );
		}
		foreach ( self::visits() as $cur ) {
			if ( $cur['id'] === $vid && $cur['last_asked'] && strtotime( $cur['last_asked'] ) > time() - 180 * DAY_IN_SECONDS ) {
				return self::err( 'nvo_cooldown', 'This patient was asked in the last 180 days.', 409 );
			}
		}
		$wpdb->insert(
			self::t( 'review_log' ),
			array(
				'visit_id'   => $vid,
				'name'       => $v->name,
				'phone'      => $v->phone,
				'email'      => $v->email,
				'channel'    => $channel,
				'user_name'  => self::staff_name( self::$staff ),
				'created_at' => self::now(),
			)
		);
		self::audit( 'visit', $vid, 'review_' . $channel, self::settings()['messaging_live'] ? 'sent' : 'simulated' );
		return self::snapshot();
	}

	/* ------------------------------------------------------------------ snapshot */

	public static function snapshot() {
		$s = self::settings();
		return array(
			'server_time' => gmdate( 'Y-m-d\TH:i:s\Z' ),
			'today'       => self::today(),
			'host'        => wp_parse_url( home_url(), PHP_URL_HOST ),
			'me'          => self::$staff ? self::staff_out( self::$staff ) : null,
			'settings'    => array(
				'review_link'        => $s['review_link'],
				'google_profile_url' => $s['google_profile_url'],
				'google_rating'      => $s['google_rating'],
				'practice_phone'     => $s['practice_phone'],
				'patient_form_url'   => $s['patient_form_url'],
				'patient_forms_on'   => (bool) $s['patient_form_id'],
				'messaging_live'     => (bool) $s['messaging_live'],
			),
			'requests'    => self::requests(),
			'forms'       => self::patient_forms(),
			'members'     => self::members(),
			'visits'      => self::visits(),
			'review_log'  => self::review_log(),
		);
	}

	public static function r_sync() {
		return self::snapshot();
	}

	/* ------------------------------------------------------------------ routes */

	public static function routes() {
		$dev   = array( __CLASS__, 'perm_device' );
		$staff = array( __CLASS__, 'perm_staff' );
		$reg   = function ( $path, $method, $cb, $perm ) {
			register_rest_route(
				self::NS,
				$path,
				array(
					'methods'             => $method,
					'callback'            => array( __CLASS__, $cb ),
					'permission_callback' => $perm,
				)
			);
		};
		$reg( '/staff', 'GET', 'r_staff', $dev );
		$reg( '/session', 'POST', 'r_login', $dev );
		$reg( '/session', 'DELETE', 'r_logout', $staff );
		$reg( '/sync', 'GET', 'r_sync', $staff );
		$reg( '/requests/(?P<id>\d+)/status', 'POST', 'r_request_status', $staff );
		$reg( '/requests/(?P<id>\d+)/viewed', 'POST', 'r_request_viewed', $staff );
		$reg( '/requests/(?P<id>\d+)/copied', 'POST', 'r_request_copied', $staff );
		$reg( '/forms/(?P<id>\d+)/attached', 'POST', 'r_form_attached', $staff );
		$reg( '/forms/(?P<id>\d+)/pdf', 'POST', 'r_form_saved_pdf', $staff );
		$reg( '/forms/link', 'POST', 'r_form_link', $staff );
		$reg( '/members', 'POST', 'r_member_add', $staff );
		$reg( '/members/(?P<id>\d+)/(?P<action>paid|renew|remind|archive)', 'POST', 'r_member_action', $staff );
		$reg( '/visits', 'POST', 'r_visit_add', $staff );
		$reg( '/reviews', 'POST', 'r_review_send', $staff );
	}

	/* ------------------------------------------------------------------ admin */

	public static function admin_menu() {
		add_management_page( 'Northview Office App', 'Northview Office App', 'manage_options', 'nvo-office', array( __CLASS__, 'admin_page' ) );
	}

	public static function admin_page() {
		if ( ! current_user_can( 'manage_options' ) ) {
			return;
		}
		$s     = self::settings();
		$users = get_users( array( 'orderby' => 'display_name' ) );
		$forms = function_exists( 'wpforms' ) ? wpforms()->form->get( '', array( 'orderby' => 'title' ) ) : array();
		?>
		<div class="wrap">
			<h1>Northview Office App</h1>
			<p>Connects the front-desk desktop app to this website. Enter the site address and device key below into the app on each office computer (Settings on the sign-in screen).</p>
			<?php if ( isset( $_GET['saved'] ) ) : ?><div class="notice notice-success"><p>Saved.</p></div><?php endif; ?>
			<form method="post" action="<?php echo esc_url( admin_url( 'admin-post.php' ) ); ?>">
				<input type="hidden" name="action" value="nvo_save">
				<?php wp_nonce_field( 'nvo_save' ); ?>
				<h2>Connection</h2>
				<table class="form-table">
					<tr><th>Site address</th><td><code><?php echo esc_html( home_url() ); ?></code></td></tr>
					<tr><th>Device key</th><td><code><?php echo esc_html( $s['device_key'] ); ?></code><br><label><input type="checkbox" name="regen" value="1"> Generate a new key (every office computer must be updated)</label></td></tr>
				</table>
				<h2>Forms and links</h2>
				<table class="form-table">
					<tr><th>Booking request form</th><td><select name="booking_form_id"><?php foreach ( $forms as $f ) : ?><option value="<?php echo (int) $f->ID; ?>" <?php selected( (int) $s['booking_form_id'], $f->ID ); ?>><?php echo esc_html( $f->post_title ); ?></option><?php endforeach; ?></select></td></tr>
					<tr><th>New-patient form</th><td><select name="patient_form_id"><option value="0">— Not set up yet —</option><?php foreach ( $forms as $f ) : ?><option value="<?php echo (int) $f->ID; ?>" <?php selected( (int) $s['patient_form_id'], $f->ID ); ?>><?php echo esc_html( $f->post_title ); ?></option><?php endforeach; ?></select><p class="description">Medical history must only be collected once hosting is covered by a HIPAA Business Associate Agreement.</p></td></tr>
					<tr><th>New-patient form page URL</th><td><input class="regular-text" name="patient_form_url" value="<?php echo esc_attr( $s['patient_form_url'] ); ?>"></td></tr>
					<tr><th>Google review link</th><td><input class="regular-text" name="review_link" value="<?php echo esc_attr( $s['review_link'] ); ?>"></td></tr>
					<tr><th>Google profile URL</th><td><input class="regular-text" name="google_profile_url" value="<?php echo esc_attr( $s['google_profile_url'] ); ?>"></td></tr>
					<tr><th>Google rating</th><td><input class="small-text" name="google_rating" value="<?php echo esc_attr( $s['google_rating'] ); ?>"></td></tr>
					<tr><th>Practice phone</th><td><input name="practice_phone" value="<?php echo esc_attr( $s['practice_phone'] ); ?>"></td></tr>
				</table>
				<h2>Staff</h2>
				<p>Tick staff who may use the app and set a 4–6 digit PIN. Leave the PIN blank to keep the current one.</p>
				<table class="widefat striped" style="max-width:900px">
					<thead><tr><th>Use app</th><th>WordPress user</th><th>Name shown in app</th><th>Role label</th><th>Manager</th><th>New PIN</th><th>PIN set</th></tr></thead>
					<tbody>
					<?php foreach ( $users as $u ) : ?>
						<tr>
							<td><input type="checkbox" name="staff[<?php echo (int) $u->ID; ?>][on]" value="1" <?php checked( (bool) get_user_meta( $u->ID, 'nvo_enabled', true ) ); ?>></td>
							<td><?php echo esc_html( $u->user_login ); ?></td>
							<td><input name="staff[<?php echo (int) $u->ID; ?>][label]" value="<?php echo esc_attr( get_user_meta( $u->ID, 'nvo_label', true ) ); ?>" placeholder="<?php echo esc_attr( $u->display_name ); ?>"></td>
							<td><input name="staff[<?php echo (int) $u->ID; ?>][role]" value="<?php echo esc_attr( get_user_meta( $u->ID, 'nvo_role_label', true ) ); ?>" placeholder="Reception"></td>
							<td><input type="checkbox" name="staff[<?php echo (int) $u->ID; ?>][manager]" value="1" <?php checked( (bool) get_user_meta( $u->ID, 'nvo_manager', true ) ); ?>></td>
							<td><input name="staff[<?php echo (int) $u->ID; ?>][pin]" value="" inputmode="numeric" pattern="\d{4,6}" autocomplete="new-password" style="width:90px"></td>
							<td><?php echo get_user_meta( $u->ID, 'nvo_pin_hash', true ) ? '✓' : '—'; ?></td>
						</tr>
					<?php endforeach; ?>
					</tbody>
				</table>
				<?php submit_button(); ?>
			</form>
		</div>
		<?php
	}

	public static function admin_save() {
		if ( ! current_user_can( 'manage_options' ) || ! check_admin_referer( 'nvo_save' ) ) {
			wp_die( 'Not allowed' );
		}
		$s = self::settings();
		foreach ( array( 'booking_form_id', 'patient_form_id' ) as $k ) {
			$s[ $k ] = isset( $_POST[ $k ] ) ? absint( $_POST[ $k ] ) : $s[ $k ];
		}
		foreach ( array( 'patient_form_url', 'review_link', 'google_profile_url' ) as $k ) {
			$s[ $k ] = isset( $_POST[ $k ] ) ? esc_url_raw( wp_unslash( $_POST[ $k ] ) ) : $s[ $k ];
		}
		foreach ( array( 'google_rating', 'practice_phone' ) as $k ) {
			$s[ $k ] = isset( $_POST[ $k ] ) ? sanitize_text_field( wp_unslash( $_POST[ $k ] ) ) : $s[ $k ];
		}
		if ( ! empty( $_POST['regen'] ) ) {
			$s['device_key'] = self::new_device_key();
		}
		update_option( self::OPT, $s, false );

		$staff = isset( $_POST['staff'] ) && is_array( $_POST['staff'] ) ? wp_unslash( $_POST['staff'] ) : array();
		foreach ( get_users( array( 'fields' => 'ID' ) ) as $uid ) {
			$row = $staff[ $uid ] ?? array();
			update_user_meta( $uid, 'nvo_enabled', empty( $row['on'] ) ? '' : '1' );
			update_user_meta( $uid, 'nvo_manager', empty( $row['manager'] ) ? '' : '1' );
			update_user_meta( $uid, 'nvo_label', sanitize_text_field( $row['label'] ?? '' ) );
			update_user_meta( $uid, 'nvo_role_label', sanitize_text_field( $row['role'] ?? '' ) );
			$pin = preg_replace( '/\D/', '', (string) ( $row['pin'] ?? '' ) );
			if ( preg_match( '/^\d{4,6}$/', $pin ) ) {
				update_user_meta( $uid, 'nvo_pin_hash', wp_hash_password( $pin ) );
				delete_transient( 'nvo_lock_' . $uid );
			}
		}
		wp_safe_redirect( admin_url( 'tools.php?page=nvo-office&saved=1' ) );
		exit;
	}
}

NVO_Office_API::boot();
