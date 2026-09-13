// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'error.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
  'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models',
);

/// @nodoc
mixin _$BridgeError {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) invalid,
    required TResult Function(String field0) accountNotFound,
    required TResult Function(int retcode, String message) api,
    required TResult Function(LoginChallengeDto field0) challenge,
    required TResult Function(String field0) storage,
    required TResult Function(String field0) capture,
    required TResult Function(String field0) qr,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? invalid,
    TResult? Function(String field0)? accountNotFound,
    TResult? Function(int retcode, String message)? api,
    TResult? Function(LoginChallengeDto field0)? challenge,
    TResult? Function(String field0)? storage,
    TResult? Function(String field0)? capture,
    TResult? Function(String field0)? qr,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? invalid,
    TResult Function(String field0)? accountNotFound,
    TResult Function(int retcode, String message)? api,
    TResult Function(LoginChallengeDto field0)? challenge,
    TResult Function(String field0)? storage,
    TResult Function(String field0)? capture,
    TResult Function(String field0)? qr,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(BridgeError_Invalid value) invalid,
    required TResult Function(BridgeError_AccountNotFound value)
    accountNotFound,
    required TResult Function(BridgeError_Api value) api,
    required TResult Function(BridgeError_Challenge value) challenge,
    required TResult Function(BridgeError_Storage value) storage,
    required TResult Function(BridgeError_Capture value) capture,
    required TResult Function(BridgeError_Qr value) qr,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(BridgeError_Invalid value)? invalid,
    TResult? Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult? Function(BridgeError_Api value)? api,
    TResult? Function(BridgeError_Challenge value)? challenge,
    TResult? Function(BridgeError_Storage value)? storage,
    TResult? Function(BridgeError_Capture value)? capture,
    TResult? Function(BridgeError_Qr value)? qr,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(BridgeError_Invalid value)? invalid,
    TResult Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult Function(BridgeError_Api value)? api,
    TResult Function(BridgeError_Challenge value)? challenge,
    TResult Function(BridgeError_Storage value)? storage,
    TResult Function(BridgeError_Capture value)? capture,
    TResult Function(BridgeError_Qr value)? qr,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $BridgeErrorCopyWith<$Res> {
  factory $BridgeErrorCopyWith(
    BridgeError value,
    $Res Function(BridgeError) then,
  ) = _$BridgeErrorCopyWithImpl<$Res, BridgeError>;
}

/// @nodoc
class _$BridgeErrorCopyWithImpl<$Res, $Val extends BridgeError>
    implements $BridgeErrorCopyWith<$Res> {
  _$BridgeErrorCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$BridgeError_InvalidImplCopyWith<$Res> {
  factory _$$BridgeError_InvalidImplCopyWith(
    _$BridgeError_InvalidImpl value,
    $Res Function(_$BridgeError_InvalidImpl) then,
  ) = __$$BridgeError_InvalidImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$BridgeError_InvalidImplCopyWithImpl<$Res>
    extends _$BridgeErrorCopyWithImpl<$Res, _$BridgeError_InvalidImpl>
    implements _$$BridgeError_InvalidImplCopyWith<$Res> {
  __$$BridgeError_InvalidImplCopyWithImpl(
    _$BridgeError_InvalidImpl _value,
    $Res Function(_$BridgeError_InvalidImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$BridgeError_InvalidImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$BridgeError_InvalidImpl extends BridgeError_Invalid {
  const _$BridgeError_InvalidImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'BridgeError.invalid(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BridgeError_InvalidImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BridgeError_InvalidImplCopyWith<_$BridgeError_InvalidImpl> get copyWith =>
      __$$BridgeError_InvalidImplCopyWithImpl<_$BridgeError_InvalidImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) invalid,
    required TResult Function(String field0) accountNotFound,
    required TResult Function(int retcode, String message) api,
    required TResult Function(LoginChallengeDto field0) challenge,
    required TResult Function(String field0) storage,
    required TResult Function(String field0) capture,
    required TResult Function(String field0) qr,
  }) {
    return invalid(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? invalid,
    TResult? Function(String field0)? accountNotFound,
    TResult? Function(int retcode, String message)? api,
    TResult? Function(LoginChallengeDto field0)? challenge,
    TResult? Function(String field0)? storage,
    TResult? Function(String field0)? capture,
    TResult? Function(String field0)? qr,
  }) {
    return invalid?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? invalid,
    TResult Function(String field0)? accountNotFound,
    TResult Function(int retcode, String message)? api,
    TResult Function(LoginChallengeDto field0)? challenge,
    TResult Function(String field0)? storage,
    TResult Function(String field0)? capture,
    TResult Function(String field0)? qr,
    required TResult orElse(),
  }) {
    if (invalid != null) {
      return invalid(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(BridgeError_Invalid value) invalid,
    required TResult Function(BridgeError_AccountNotFound value)
    accountNotFound,
    required TResult Function(BridgeError_Api value) api,
    required TResult Function(BridgeError_Challenge value) challenge,
    required TResult Function(BridgeError_Storage value) storage,
    required TResult Function(BridgeError_Capture value) capture,
    required TResult Function(BridgeError_Qr value) qr,
  }) {
    return invalid(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(BridgeError_Invalid value)? invalid,
    TResult? Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult? Function(BridgeError_Api value)? api,
    TResult? Function(BridgeError_Challenge value)? challenge,
    TResult? Function(BridgeError_Storage value)? storage,
    TResult? Function(BridgeError_Capture value)? capture,
    TResult? Function(BridgeError_Qr value)? qr,
  }) {
    return invalid?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(BridgeError_Invalid value)? invalid,
    TResult Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult Function(BridgeError_Api value)? api,
    TResult Function(BridgeError_Challenge value)? challenge,
    TResult Function(BridgeError_Storage value)? storage,
    TResult Function(BridgeError_Capture value)? capture,
    TResult Function(BridgeError_Qr value)? qr,
    required TResult orElse(),
  }) {
    if (invalid != null) {
      return invalid(this);
    }
    return orElse();
  }
}

abstract class BridgeError_Invalid extends BridgeError {
  const factory BridgeError_Invalid(String field0) = _$BridgeError_InvalidImpl;
  const BridgeError_Invalid._() : super._();

  String get field0;

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BridgeError_InvalidImplCopyWith<_$BridgeError_InvalidImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BridgeError_AccountNotFoundImplCopyWith<$Res> {
  factory _$$BridgeError_AccountNotFoundImplCopyWith(
    _$BridgeError_AccountNotFoundImpl value,
    $Res Function(_$BridgeError_AccountNotFoundImpl) then,
  ) = __$$BridgeError_AccountNotFoundImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$BridgeError_AccountNotFoundImplCopyWithImpl<$Res>
    extends _$BridgeErrorCopyWithImpl<$Res, _$BridgeError_AccountNotFoundImpl>
    implements _$$BridgeError_AccountNotFoundImplCopyWith<$Res> {
  __$$BridgeError_AccountNotFoundImplCopyWithImpl(
    _$BridgeError_AccountNotFoundImpl _value,
    $Res Function(_$BridgeError_AccountNotFoundImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$BridgeError_AccountNotFoundImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$BridgeError_AccountNotFoundImpl extends BridgeError_AccountNotFound {
  const _$BridgeError_AccountNotFoundImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'BridgeError.accountNotFound(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BridgeError_AccountNotFoundImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BridgeError_AccountNotFoundImplCopyWith<_$BridgeError_AccountNotFoundImpl>
  get copyWith =>
      __$$BridgeError_AccountNotFoundImplCopyWithImpl<
        _$BridgeError_AccountNotFoundImpl
      >(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) invalid,
    required TResult Function(String field0) accountNotFound,
    required TResult Function(int retcode, String message) api,
    required TResult Function(LoginChallengeDto field0) challenge,
    required TResult Function(String field0) storage,
    required TResult Function(String field0) capture,
    required TResult Function(String field0) qr,
  }) {
    return accountNotFound(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? invalid,
    TResult? Function(String field0)? accountNotFound,
    TResult? Function(int retcode, String message)? api,
    TResult? Function(LoginChallengeDto field0)? challenge,
    TResult? Function(String field0)? storage,
    TResult? Function(String field0)? capture,
    TResult? Function(String field0)? qr,
  }) {
    return accountNotFound?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? invalid,
    TResult Function(String field0)? accountNotFound,
    TResult Function(int retcode, String message)? api,
    TResult Function(LoginChallengeDto field0)? challenge,
    TResult Function(String field0)? storage,
    TResult Function(String field0)? capture,
    TResult Function(String field0)? qr,
    required TResult orElse(),
  }) {
    if (accountNotFound != null) {
      return accountNotFound(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(BridgeError_Invalid value) invalid,
    required TResult Function(BridgeError_AccountNotFound value)
    accountNotFound,
    required TResult Function(BridgeError_Api value) api,
    required TResult Function(BridgeError_Challenge value) challenge,
    required TResult Function(BridgeError_Storage value) storage,
    required TResult Function(BridgeError_Capture value) capture,
    required TResult Function(BridgeError_Qr value) qr,
  }) {
    return accountNotFound(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(BridgeError_Invalid value)? invalid,
    TResult? Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult? Function(BridgeError_Api value)? api,
    TResult? Function(BridgeError_Challenge value)? challenge,
    TResult? Function(BridgeError_Storage value)? storage,
    TResult? Function(BridgeError_Capture value)? capture,
    TResult? Function(BridgeError_Qr value)? qr,
  }) {
    return accountNotFound?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(BridgeError_Invalid value)? invalid,
    TResult Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult Function(BridgeError_Api value)? api,
    TResult Function(BridgeError_Challenge value)? challenge,
    TResult Function(BridgeError_Storage value)? storage,
    TResult Function(BridgeError_Capture value)? capture,
    TResult Function(BridgeError_Qr value)? qr,
    required TResult orElse(),
  }) {
    if (accountNotFound != null) {
      return accountNotFound(this);
    }
    return orElse();
  }
}

abstract class BridgeError_AccountNotFound extends BridgeError {
  const factory BridgeError_AccountNotFound(String field0) =
      _$BridgeError_AccountNotFoundImpl;
  const BridgeError_AccountNotFound._() : super._();

  String get field0;

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BridgeError_AccountNotFoundImplCopyWith<_$BridgeError_AccountNotFoundImpl>
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BridgeError_ApiImplCopyWith<$Res> {
  factory _$$BridgeError_ApiImplCopyWith(
    _$BridgeError_ApiImpl value,
    $Res Function(_$BridgeError_ApiImpl) then,
  ) = __$$BridgeError_ApiImplCopyWithImpl<$Res>;
  @useResult
  $Res call({int retcode, String message});
}

/// @nodoc
class __$$BridgeError_ApiImplCopyWithImpl<$Res>
    extends _$BridgeErrorCopyWithImpl<$Res, _$BridgeError_ApiImpl>
    implements _$$BridgeError_ApiImplCopyWith<$Res> {
  __$$BridgeError_ApiImplCopyWithImpl(
    _$BridgeError_ApiImpl _value,
    $Res Function(_$BridgeError_ApiImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? retcode = null, Object? message = null}) {
    return _then(
      _$BridgeError_ApiImpl(
        retcode: null == retcode
            ? _value.retcode
            : retcode // ignore: cast_nullable_to_non_nullable
                  as int,
        message: null == message
            ? _value.message
            : message // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$BridgeError_ApiImpl extends BridgeError_Api {
  const _$BridgeError_ApiImpl({required this.retcode, required this.message})
    : super._();

  @override
  final int retcode;
  @override
  final String message;

  @override
  String toString() {
    return 'BridgeError.api(retcode: $retcode, message: $message)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BridgeError_ApiImpl &&
            (identical(other.retcode, retcode) || other.retcode == retcode) &&
            (identical(other.message, message) || other.message == message));
  }

  @override
  int get hashCode => Object.hash(runtimeType, retcode, message);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BridgeError_ApiImplCopyWith<_$BridgeError_ApiImpl> get copyWith =>
      __$$BridgeError_ApiImplCopyWithImpl<_$BridgeError_ApiImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) invalid,
    required TResult Function(String field0) accountNotFound,
    required TResult Function(int retcode, String message) api,
    required TResult Function(LoginChallengeDto field0) challenge,
    required TResult Function(String field0) storage,
    required TResult Function(String field0) capture,
    required TResult Function(String field0) qr,
  }) {
    return api(retcode, message);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? invalid,
    TResult? Function(String field0)? accountNotFound,
    TResult? Function(int retcode, String message)? api,
    TResult? Function(LoginChallengeDto field0)? challenge,
    TResult? Function(String field0)? storage,
    TResult? Function(String field0)? capture,
    TResult? Function(String field0)? qr,
  }) {
    return api?.call(retcode, message);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? invalid,
    TResult Function(String field0)? accountNotFound,
    TResult Function(int retcode, String message)? api,
    TResult Function(LoginChallengeDto field0)? challenge,
    TResult Function(String field0)? storage,
    TResult Function(String field0)? capture,
    TResult Function(String field0)? qr,
    required TResult orElse(),
  }) {
    if (api != null) {
      return api(retcode, message);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(BridgeError_Invalid value) invalid,
    required TResult Function(BridgeError_AccountNotFound value)
    accountNotFound,
    required TResult Function(BridgeError_Api value) api,
    required TResult Function(BridgeError_Challenge value) challenge,
    required TResult Function(BridgeError_Storage value) storage,
    required TResult Function(BridgeError_Capture value) capture,
    required TResult Function(BridgeError_Qr value) qr,
  }) {
    return api(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(BridgeError_Invalid value)? invalid,
    TResult? Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult? Function(BridgeError_Api value)? api,
    TResult? Function(BridgeError_Challenge value)? challenge,
    TResult? Function(BridgeError_Storage value)? storage,
    TResult? Function(BridgeError_Capture value)? capture,
    TResult? Function(BridgeError_Qr value)? qr,
  }) {
    return api?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(BridgeError_Invalid value)? invalid,
    TResult Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult Function(BridgeError_Api value)? api,
    TResult Function(BridgeError_Challenge value)? challenge,
    TResult Function(BridgeError_Storage value)? storage,
    TResult Function(BridgeError_Capture value)? capture,
    TResult Function(BridgeError_Qr value)? qr,
    required TResult orElse(),
  }) {
    if (api != null) {
      return api(this);
    }
    return orElse();
  }
}

abstract class BridgeError_Api extends BridgeError {
  const factory BridgeError_Api({
    required int retcode,
    required String message,
  }) = _$BridgeError_ApiImpl;
  const BridgeError_Api._() : super._();

  int get retcode;
  String get message;

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BridgeError_ApiImplCopyWith<_$BridgeError_ApiImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BridgeError_ChallengeImplCopyWith<$Res> {
  factory _$$BridgeError_ChallengeImplCopyWith(
    _$BridgeError_ChallengeImpl value,
    $Res Function(_$BridgeError_ChallengeImpl) then,
  ) = __$$BridgeError_ChallengeImplCopyWithImpl<$Res>;
  @useResult
  $Res call({LoginChallengeDto field0});
}

/// @nodoc
class __$$BridgeError_ChallengeImplCopyWithImpl<$Res>
    extends _$BridgeErrorCopyWithImpl<$Res, _$BridgeError_ChallengeImpl>
    implements _$$BridgeError_ChallengeImplCopyWith<$Res> {
  __$$BridgeError_ChallengeImplCopyWithImpl(
    _$BridgeError_ChallengeImpl _value,
    $Res Function(_$BridgeError_ChallengeImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$BridgeError_ChallengeImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as LoginChallengeDto,
      ),
    );
  }
}

/// @nodoc

class _$BridgeError_ChallengeImpl extends BridgeError_Challenge {
  const _$BridgeError_ChallengeImpl(this.field0) : super._();

  @override
  final LoginChallengeDto field0;

  @override
  String toString() {
    return 'BridgeError.challenge(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BridgeError_ChallengeImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BridgeError_ChallengeImplCopyWith<_$BridgeError_ChallengeImpl>
  get copyWith =>
      __$$BridgeError_ChallengeImplCopyWithImpl<_$BridgeError_ChallengeImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) invalid,
    required TResult Function(String field0) accountNotFound,
    required TResult Function(int retcode, String message) api,
    required TResult Function(LoginChallengeDto field0) challenge,
    required TResult Function(String field0) storage,
    required TResult Function(String field0) capture,
    required TResult Function(String field0) qr,
  }) {
    return challenge(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? invalid,
    TResult? Function(String field0)? accountNotFound,
    TResult? Function(int retcode, String message)? api,
    TResult? Function(LoginChallengeDto field0)? challenge,
    TResult? Function(String field0)? storage,
    TResult? Function(String field0)? capture,
    TResult? Function(String field0)? qr,
  }) {
    return challenge?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? invalid,
    TResult Function(String field0)? accountNotFound,
    TResult Function(int retcode, String message)? api,
    TResult Function(LoginChallengeDto field0)? challenge,
    TResult Function(String field0)? storage,
    TResult Function(String field0)? capture,
    TResult Function(String field0)? qr,
    required TResult orElse(),
  }) {
    if (challenge != null) {
      return challenge(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(BridgeError_Invalid value) invalid,
    required TResult Function(BridgeError_AccountNotFound value)
    accountNotFound,
    required TResult Function(BridgeError_Api value) api,
    required TResult Function(BridgeError_Challenge value) challenge,
    required TResult Function(BridgeError_Storage value) storage,
    required TResult Function(BridgeError_Capture value) capture,
    required TResult Function(BridgeError_Qr value) qr,
  }) {
    return challenge(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(BridgeError_Invalid value)? invalid,
    TResult? Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult? Function(BridgeError_Api value)? api,
    TResult? Function(BridgeError_Challenge value)? challenge,
    TResult? Function(BridgeError_Storage value)? storage,
    TResult? Function(BridgeError_Capture value)? capture,
    TResult? Function(BridgeError_Qr value)? qr,
  }) {
    return challenge?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(BridgeError_Invalid value)? invalid,
    TResult Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult Function(BridgeError_Api value)? api,
    TResult Function(BridgeError_Challenge value)? challenge,
    TResult Function(BridgeError_Storage value)? storage,
    TResult Function(BridgeError_Capture value)? capture,
    TResult Function(BridgeError_Qr value)? qr,
    required TResult orElse(),
  }) {
    if (challenge != null) {
      return challenge(this);
    }
    return orElse();
  }
}

abstract class BridgeError_Challenge extends BridgeError {
  const factory BridgeError_Challenge(LoginChallengeDto field0) =
      _$BridgeError_ChallengeImpl;
  const BridgeError_Challenge._() : super._();

  LoginChallengeDto get field0;

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BridgeError_ChallengeImplCopyWith<_$BridgeError_ChallengeImpl>
  get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BridgeError_StorageImplCopyWith<$Res> {
  factory _$$BridgeError_StorageImplCopyWith(
    _$BridgeError_StorageImpl value,
    $Res Function(_$BridgeError_StorageImpl) then,
  ) = __$$BridgeError_StorageImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$BridgeError_StorageImplCopyWithImpl<$Res>
    extends _$BridgeErrorCopyWithImpl<$Res, _$BridgeError_StorageImpl>
    implements _$$BridgeError_StorageImplCopyWith<$Res> {
  __$$BridgeError_StorageImplCopyWithImpl(
    _$BridgeError_StorageImpl _value,
    $Res Function(_$BridgeError_StorageImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$BridgeError_StorageImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$BridgeError_StorageImpl extends BridgeError_Storage {
  const _$BridgeError_StorageImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'BridgeError.storage(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BridgeError_StorageImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BridgeError_StorageImplCopyWith<_$BridgeError_StorageImpl> get copyWith =>
      __$$BridgeError_StorageImplCopyWithImpl<_$BridgeError_StorageImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) invalid,
    required TResult Function(String field0) accountNotFound,
    required TResult Function(int retcode, String message) api,
    required TResult Function(LoginChallengeDto field0) challenge,
    required TResult Function(String field0) storage,
    required TResult Function(String field0) capture,
    required TResult Function(String field0) qr,
  }) {
    return storage(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? invalid,
    TResult? Function(String field0)? accountNotFound,
    TResult? Function(int retcode, String message)? api,
    TResult? Function(LoginChallengeDto field0)? challenge,
    TResult? Function(String field0)? storage,
    TResult? Function(String field0)? capture,
    TResult? Function(String field0)? qr,
  }) {
    return storage?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? invalid,
    TResult Function(String field0)? accountNotFound,
    TResult Function(int retcode, String message)? api,
    TResult Function(LoginChallengeDto field0)? challenge,
    TResult Function(String field0)? storage,
    TResult Function(String field0)? capture,
    TResult Function(String field0)? qr,
    required TResult orElse(),
  }) {
    if (storage != null) {
      return storage(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(BridgeError_Invalid value) invalid,
    required TResult Function(BridgeError_AccountNotFound value)
    accountNotFound,
    required TResult Function(BridgeError_Api value) api,
    required TResult Function(BridgeError_Challenge value) challenge,
    required TResult Function(BridgeError_Storage value) storage,
    required TResult Function(BridgeError_Capture value) capture,
    required TResult Function(BridgeError_Qr value) qr,
  }) {
    return storage(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(BridgeError_Invalid value)? invalid,
    TResult? Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult? Function(BridgeError_Api value)? api,
    TResult? Function(BridgeError_Challenge value)? challenge,
    TResult? Function(BridgeError_Storage value)? storage,
    TResult? Function(BridgeError_Capture value)? capture,
    TResult? Function(BridgeError_Qr value)? qr,
  }) {
    return storage?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(BridgeError_Invalid value)? invalid,
    TResult Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult Function(BridgeError_Api value)? api,
    TResult Function(BridgeError_Challenge value)? challenge,
    TResult Function(BridgeError_Storage value)? storage,
    TResult Function(BridgeError_Capture value)? capture,
    TResult Function(BridgeError_Qr value)? qr,
    required TResult orElse(),
  }) {
    if (storage != null) {
      return storage(this);
    }
    return orElse();
  }
}

abstract class BridgeError_Storage extends BridgeError {
  const factory BridgeError_Storage(String field0) = _$BridgeError_StorageImpl;
  const BridgeError_Storage._() : super._();

  String get field0;

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BridgeError_StorageImplCopyWith<_$BridgeError_StorageImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BridgeError_CaptureImplCopyWith<$Res> {
  factory _$$BridgeError_CaptureImplCopyWith(
    _$BridgeError_CaptureImpl value,
    $Res Function(_$BridgeError_CaptureImpl) then,
  ) = __$$BridgeError_CaptureImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$BridgeError_CaptureImplCopyWithImpl<$Res>
    extends _$BridgeErrorCopyWithImpl<$Res, _$BridgeError_CaptureImpl>
    implements _$$BridgeError_CaptureImplCopyWith<$Res> {
  __$$BridgeError_CaptureImplCopyWithImpl(
    _$BridgeError_CaptureImpl _value,
    $Res Function(_$BridgeError_CaptureImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$BridgeError_CaptureImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$BridgeError_CaptureImpl extends BridgeError_Capture {
  const _$BridgeError_CaptureImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'BridgeError.capture(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BridgeError_CaptureImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BridgeError_CaptureImplCopyWith<_$BridgeError_CaptureImpl> get copyWith =>
      __$$BridgeError_CaptureImplCopyWithImpl<_$BridgeError_CaptureImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) invalid,
    required TResult Function(String field0) accountNotFound,
    required TResult Function(int retcode, String message) api,
    required TResult Function(LoginChallengeDto field0) challenge,
    required TResult Function(String field0) storage,
    required TResult Function(String field0) capture,
    required TResult Function(String field0) qr,
  }) {
    return capture(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? invalid,
    TResult? Function(String field0)? accountNotFound,
    TResult? Function(int retcode, String message)? api,
    TResult? Function(LoginChallengeDto field0)? challenge,
    TResult? Function(String field0)? storage,
    TResult? Function(String field0)? capture,
    TResult? Function(String field0)? qr,
  }) {
    return capture?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? invalid,
    TResult Function(String field0)? accountNotFound,
    TResult Function(int retcode, String message)? api,
    TResult Function(LoginChallengeDto field0)? challenge,
    TResult Function(String field0)? storage,
    TResult Function(String field0)? capture,
    TResult Function(String field0)? qr,
    required TResult orElse(),
  }) {
    if (capture != null) {
      return capture(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(BridgeError_Invalid value) invalid,
    required TResult Function(BridgeError_AccountNotFound value)
    accountNotFound,
    required TResult Function(BridgeError_Api value) api,
    required TResult Function(BridgeError_Challenge value) challenge,
    required TResult Function(BridgeError_Storage value) storage,
    required TResult Function(BridgeError_Capture value) capture,
    required TResult Function(BridgeError_Qr value) qr,
  }) {
    return capture(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(BridgeError_Invalid value)? invalid,
    TResult? Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult? Function(BridgeError_Api value)? api,
    TResult? Function(BridgeError_Challenge value)? challenge,
    TResult? Function(BridgeError_Storage value)? storage,
    TResult? Function(BridgeError_Capture value)? capture,
    TResult? Function(BridgeError_Qr value)? qr,
  }) {
    return capture?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(BridgeError_Invalid value)? invalid,
    TResult Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult Function(BridgeError_Api value)? api,
    TResult Function(BridgeError_Challenge value)? challenge,
    TResult Function(BridgeError_Storage value)? storage,
    TResult Function(BridgeError_Capture value)? capture,
    TResult Function(BridgeError_Qr value)? qr,
    required TResult orElse(),
  }) {
    if (capture != null) {
      return capture(this);
    }
    return orElse();
  }
}

abstract class BridgeError_Capture extends BridgeError {
  const factory BridgeError_Capture(String field0) = _$BridgeError_CaptureImpl;
  const BridgeError_Capture._() : super._();

  String get field0;

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BridgeError_CaptureImplCopyWith<_$BridgeError_CaptureImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$BridgeError_QrImplCopyWith<$Res> {
  factory _$$BridgeError_QrImplCopyWith(
    _$BridgeError_QrImpl value,
    $Res Function(_$BridgeError_QrImpl) then,
  ) = __$$BridgeError_QrImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$BridgeError_QrImplCopyWithImpl<$Res>
    extends _$BridgeErrorCopyWithImpl<$Res, _$BridgeError_QrImpl>
    implements _$$BridgeError_QrImplCopyWith<$Res> {
  __$$BridgeError_QrImplCopyWithImpl(
    _$BridgeError_QrImpl _value,
    $Res Function(_$BridgeError_QrImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$BridgeError_QrImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$BridgeError_QrImpl extends BridgeError_Qr {
  const _$BridgeError_QrImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'BridgeError.qr(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$BridgeError_QrImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$BridgeError_QrImplCopyWith<_$BridgeError_QrImpl> get copyWith =>
      __$$BridgeError_QrImplCopyWithImpl<_$BridgeError_QrImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) invalid,
    required TResult Function(String field0) accountNotFound,
    required TResult Function(int retcode, String message) api,
    required TResult Function(LoginChallengeDto field0) challenge,
    required TResult Function(String field0) storage,
    required TResult Function(String field0) capture,
    required TResult Function(String field0) qr,
  }) {
    return qr(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? invalid,
    TResult? Function(String field0)? accountNotFound,
    TResult? Function(int retcode, String message)? api,
    TResult? Function(LoginChallengeDto field0)? challenge,
    TResult? Function(String field0)? storage,
    TResult? Function(String field0)? capture,
    TResult? Function(String field0)? qr,
  }) {
    return qr?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? invalid,
    TResult Function(String field0)? accountNotFound,
    TResult Function(int retcode, String message)? api,
    TResult Function(LoginChallengeDto field0)? challenge,
    TResult Function(String field0)? storage,
    TResult Function(String field0)? capture,
    TResult Function(String field0)? qr,
    required TResult orElse(),
  }) {
    if (qr != null) {
      return qr(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(BridgeError_Invalid value) invalid,
    required TResult Function(BridgeError_AccountNotFound value)
    accountNotFound,
    required TResult Function(BridgeError_Api value) api,
    required TResult Function(BridgeError_Challenge value) challenge,
    required TResult Function(BridgeError_Storage value) storage,
    required TResult Function(BridgeError_Capture value) capture,
    required TResult Function(BridgeError_Qr value) qr,
  }) {
    return qr(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(BridgeError_Invalid value)? invalid,
    TResult? Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult? Function(BridgeError_Api value)? api,
    TResult? Function(BridgeError_Challenge value)? challenge,
    TResult? Function(BridgeError_Storage value)? storage,
    TResult? Function(BridgeError_Capture value)? capture,
    TResult? Function(BridgeError_Qr value)? qr,
  }) {
    return qr?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(BridgeError_Invalid value)? invalid,
    TResult Function(BridgeError_AccountNotFound value)? accountNotFound,
    TResult Function(BridgeError_Api value)? api,
    TResult Function(BridgeError_Challenge value)? challenge,
    TResult Function(BridgeError_Storage value)? storage,
    TResult Function(BridgeError_Capture value)? capture,
    TResult Function(BridgeError_Qr value)? qr,
    required TResult orElse(),
  }) {
    if (qr != null) {
      return qr(this);
    }
    return orElse();
  }
}

abstract class BridgeError_Qr extends BridgeError {
  const factory BridgeError_Qr(String field0) = _$BridgeError_QrImpl;
  const BridgeError_Qr._() : super._();

  String get field0;

  /// Create a copy of BridgeError
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$BridgeError_QrImplCopyWith<_$BridgeError_QrImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
