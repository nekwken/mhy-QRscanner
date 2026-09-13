// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'qr.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
  'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#adding-getters-and-methods-to-our-models',
);

/// @nodoc
mixin _$QrSource {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) url,
    required TResult Function(String field0) image,
    required TResult Function(BigInt? waitSecs) screen,
    required TResult Function(String roomId, BigInt? waitSecs) bilibili,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? url,
    TResult? Function(String field0)? image,
    TResult? Function(BigInt? waitSecs)? screen,
    TResult? Function(String roomId, BigInt? waitSecs)? bilibili,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? url,
    TResult Function(String field0)? image,
    TResult Function(BigInt? waitSecs)? screen,
    TResult Function(String roomId, BigInt? waitSecs)? bilibili,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(QrSource_Url value) url,
    required TResult Function(QrSource_Image value) image,
    required TResult Function(QrSource_Screen value) screen,
    required TResult Function(QrSource_Bilibili value) bilibili,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(QrSource_Url value)? url,
    TResult? Function(QrSource_Image value)? image,
    TResult? Function(QrSource_Screen value)? screen,
    TResult? Function(QrSource_Bilibili value)? bilibili,
  }) => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(QrSource_Url value)? url,
    TResult Function(QrSource_Image value)? image,
    TResult Function(QrSource_Screen value)? screen,
    TResult Function(QrSource_Bilibili value)? bilibili,
    required TResult orElse(),
  }) => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $QrSourceCopyWith<$Res> {
  factory $QrSourceCopyWith(QrSource value, $Res Function(QrSource) then) =
      _$QrSourceCopyWithImpl<$Res, QrSource>;
}

/// @nodoc
class _$QrSourceCopyWithImpl<$Res, $Val extends QrSource>
    implements $QrSourceCopyWith<$Res> {
  _$QrSourceCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
}

/// @nodoc
abstract class _$$QrSource_UrlImplCopyWith<$Res> {
  factory _$$QrSource_UrlImplCopyWith(
    _$QrSource_UrlImpl value,
    $Res Function(_$QrSource_UrlImpl) then,
  ) = __$$QrSource_UrlImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$QrSource_UrlImplCopyWithImpl<$Res>
    extends _$QrSourceCopyWithImpl<$Res, _$QrSource_UrlImpl>
    implements _$$QrSource_UrlImplCopyWith<$Res> {
  __$$QrSource_UrlImplCopyWithImpl(
    _$QrSource_UrlImpl _value,
    $Res Function(_$QrSource_UrlImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$QrSource_UrlImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$QrSource_UrlImpl extends QrSource_Url {
  const _$QrSource_UrlImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'QrSource.url(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$QrSource_UrlImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$QrSource_UrlImplCopyWith<_$QrSource_UrlImpl> get copyWith =>
      __$$QrSource_UrlImplCopyWithImpl<_$QrSource_UrlImpl>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) url,
    required TResult Function(String field0) image,
    required TResult Function(BigInt? waitSecs) screen,
    required TResult Function(String roomId, BigInt? waitSecs) bilibili,
  }) {
    return url(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? url,
    TResult? Function(String field0)? image,
    TResult? Function(BigInt? waitSecs)? screen,
    TResult? Function(String roomId, BigInt? waitSecs)? bilibili,
  }) {
    return url?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? url,
    TResult Function(String field0)? image,
    TResult Function(BigInt? waitSecs)? screen,
    TResult Function(String roomId, BigInt? waitSecs)? bilibili,
    required TResult orElse(),
  }) {
    if (url != null) {
      return url(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(QrSource_Url value) url,
    required TResult Function(QrSource_Image value) image,
    required TResult Function(QrSource_Screen value) screen,
    required TResult Function(QrSource_Bilibili value) bilibili,
  }) {
    return url(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(QrSource_Url value)? url,
    TResult? Function(QrSource_Image value)? image,
    TResult? Function(QrSource_Screen value)? screen,
    TResult? Function(QrSource_Bilibili value)? bilibili,
  }) {
    return url?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(QrSource_Url value)? url,
    TResult Function(QrSource_Image value)? image,
    TResult Function(QrSource_Screen value)? screen,
    TResult Function(QrSource_Bilibili value)? bilibili,
    required TResult orElse(),
  }) {
    if (url != null) {
      return url(this);
    }
    return orElse();
  }
}

abstract class QrSource_Url extends QrSource {
  const factory QrSource_Url(String field0) = _$QrSource_UrlImpl;
  const QrSource_Url._() : super._();

  String get field0;

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$QrSource_UrlImplCopyWith<_$QrSource_UrlImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$QrSource_ImageImplCopyWith<$Res> {
  factory _$$QrSource_ImageImplCopyWith(
    _$QrSource_ImageImpl value,
    $Res Function(_$QrSource_ImageImpl) then,
  ) = __$$QrSource_ImageImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String field0});
}

/// @nodoc
class __$$QrSource_ImageImplCopyWithImpl<$Res>
    extends _$QrSourceCopyWithImpl<$Res, _$QrSource_ImageImpl>
    implements _$$QrSource_ImageImplCopyWith<$Res> {
  __$$QrSource_ImageImplCopyWithImpl(
    _$QrSource_ImageImpl _value,
    $Res Function(_$QrSource_ImageImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? field0 = null}) {
    return _then(
      _$QrSource_ImageImpl(
        null == field0
            ? _value.field0
            : field0 // ignore: cast_nullable_to_non_nullable
                  as String,
      ),
    );
  }
}

/// @nodoc

class _$QrSource_ImageImpl extends QrSource_Image {
  const _$QrSource_ImageImpl(this.field0) : super._();

  @override
  final String field0;

  @override
  String toString() {
    return 'QrSource.image(field0: $field0)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$QrSource_ImageImpl &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$QrSource_ImageImplCopyWith<_$QrSource_ImageImpl> get copyWith =>
      __$$QrSource_ImageImplCopyWithImpl<_$QrSource_ImageImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) url,
    required TResult Function(String field0) image,
    required TResult Function(BigInt? waitSecs) screen,
    required TResult Function(String roomId, BigInt? waitSecs) bilibili,
  }) {
    return image(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? url,
    TResult? Function(String field0)? image,
    TResult? Function(BigInt? waitSecs)? screen,
    TResult? Function(String roomId, BigInt? waitSecs)? bilibili,
  }) {
    return image?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? url,
    TResult Function(String field0)? image,
    TResult Function(BigInt? waitSecs)? screen,
    TResult Function(String roomId, BigInt? waitSecs)? bilibili,
    required TResult orElse(),
  }) {
    if (image != null) {
      return image(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(QrSource_Url value) url,
    required TResult Function(QrSource_Image value) image,
    required TResult Function(QrSource_Screen value) screen,
    required TResult Function(QrSource_Bilibili value) bilibili,
  }) {
    return image(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(QrSource_Url value)? url,
    TResult? Function(QrSource_Image value)? image,
    TResult? Function(QrSource_Screen value)? screen,
    TResult? Function(QrSource_Bilibili value)? bilibili,
  }) {
    return image?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(QrSource_Url value)? url,
    TResult Function(QrSource_Image value)? image,
    TResult Function(QrSource_Screen value)? screen,
    TResult Function(QrSource_Bilibili value)? bilibili,
    required TResult orElse(),
  }) {
    if (image != null) {
      return image(this);
    }
    return orElse();
  }
}

abstract class QrSource_Image extends QrSource {
  const factory QrSource_Image(String field0) = _$QrSource_ImageImpl;
  const QrSource_Image._() : super._();

  String get field0;

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$QrSource_ImageImplCopyWith<_$QrSource_ImageImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$QrSource_ScreenImplCopyWith<$Res> {
  factory _$$QrSource_ScreenImplCopyWith(
    _$QrSource_ScreenImpl value,
    $Res Function(_$QrSource_ScreenImpl) then,
  ) = __$$QrSource_ScreenImplCopyWithImpl<$Res>;
  @useResult
  $Res call({BigInt? waitSecs});
}

/// @nodoc
class __$$QrSource_ScreenImplCopyWithImpl<$Res>
    extends _$QrSourceCopyWithImpl<$Res, _$QrSource_ScreenImpl>
    implements _$$QrSource_ScreenImplCopyWith<$Res> {
  __$$QrSource_ScreenImplCopyWithImpl(
    _$QrSource_ScreenImpl _value,
    $Res Function(_$QrSource_ScreenImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? waitSecs = freezed}) {
    return _then(
      _$QrSource_ScreenImpl(
        waitSecs: freezed == waitSecs
            ? _value.waitSecs
            : waitSecs // ignore: cast_nullable_to_non_nullable
                  as BigInt?,
      ),
    );
  }
}

/// @nodoc

class _$QrSource_ScreenImpl extends QrSource_Screen {
  const _$QrSource_ScreenImpl({this.waitSecs}) : super._();

  @override
  final BigInt? waitSecs;

  @override
  String toString() {
    return 'QrSource.screen(waitSecs: $waitSecs)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$QrSource_ScreenImpl &&
            (identical(other.waitSecs, waitSecs) ||
                other.waitSecs == waitSecs));
  }

  @override
  int get hashCode => Object.hash(runtimeType, waitSecs);

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$QrSource_ScreenImplCopyWith<_$QrSource_ScreenImpl> get copyWith =>
      __$$QrSource_ScreenImplCopyWithImpl<_$QrSource_ScreenImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) url,
    required TResult Function(String field0) image,
    required TResult Function(BigInt? waitSecs) screen,
    required TResult Function(String roomId, BigInt? waitSecs) bilibili,
  }) {
    return screen(waitSecs);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? url,
    TResult? Function(String field0)? image,
    TResult? Function(BigInt? waitSecs)? screen,
    TResult? Function(String roomId, BigInt? waitSecs)? bilibili,
  }) {
    return screen?.call(waitSecs);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? url,
    TResult Function(String field0)? image,
    TResult Function(BigInt? waitSecs)? screen,
    TResult Function(String roomId, BigInt? waitSecs)? bilibili,
    required TResult orElse(),
  }) {
    if (screen != null) {
      return screen(waitSecs);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(QrSource_Url value) url,
    required TResult Function(QrSource_Image value) image,
    required TResult Function(QrSource_Screen value) screen,
    required TResult Function(QrSource_Bilibili value) bilibili,
  }) {
    return screen(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(QrSource_Url value)? url,
    TResult? Function(QrSource_Image value)? image,
    TResult? Function(QrSource_Screen value)? screen,
    TResult? Function(QrSource_Bilibili value)? bilibili,
  }) {
    return screen?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(QrSource_Url value)? url,
    TResult Function(QrSource_Image value)? image,
    TResult Function(QrSource_Screen value)? screen,
    TResult Function(QrSource_Bilibili value)? bilibili,
    required TResult orElse(),
  }) {
    if (screen != null) {
      return screen(this);
    }
    return orElse();
  }
}

abstract class QrSource_Screen extends QrSource {
  const factory QrSource_Screen({BigInt? waitSecs}) = _$QrSource_ScreenImpl;
  const QrSource_Screen._() : super._();

  BigInt? get waitSecs;

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$QrSource_ScreenImplCopyWith<_$QrSource_ScreenImpl> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$QrSource_BilibiliImplCopyWith<$Res> {
  factory _$$QrSource_BilibiliImplCopyWith(
    _$QrSource_BilibiliImpl value,
    $Res Function(_$QrSource_BilibiliImpl) then,
  ) = __$$QrSource_BilibiliImplCopyWithImpl<$Res>;
  @useResult
  $Res call({String roomId, BigInt? waitSecs});
}

/// @nodoc
class __$$QrSource_BilibiliImplCopyWithImpl<$Res>
    extends _$QrSourceCopyWithImpl<$Res, _$QrSource_BilibiliImpl>
    implements _$$QrSource_BilibiliImplCopyWith<$Res> {
  __$$QrSource_BilibiliImplCopyWithImpl(
    _$QrSource_BilibiliImpl _value,
    $Res Function(_$QrSource_BilibiliImpl) _then,
  ) : super(_value, _then);

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @pragma('vm:prefer-inline')
  @override
  $Res call({Object? roomId = null, Object? waitSecs = freezed}) {
    return _then(
      _$QrSource_BilibiliImpl(
        roomId: null == roomId
            ? _value.roomId
            : roomId // ignore: cast_nullable_to_non_nullable
                  as String,
        waitSecs: freezed == waitSecs
            ? _value.waitSecs
            : waitSecs // ignore: cast_nullable_to_non_nullable
                  as BigInt?,
      ),
    );
  }
}

/// @nodoc

class _$QrSource_BilibiliImpl extends QrSource_Bilibili {
  const _$QrSource_BilibiliImpl({required this.roomId, this.waitSecs})
    : super._();

  @override
  final String roomId;
  @override
  final BigInt? waitSecs;

  @override
  String toString() {
    return 'QrSource.bilibili(roomId: $roomId, waitSecs: $waitSecs)';
  }

  @override
  bool operator ==(Object other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$QrSource_BilibiliImpl &&
            (identical(other.roomId, roomId) || other.roomId == roomId) &&
            (identical(other.waitSecs, waitSecs) ||
                other.waitSecs == waitSecs));
  }

  @override
  int get hashCode => Object.hash(runtimeType, roomId, waitSecs);

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  @override
  @pragma('vm:prefer-inline')
  _$$QrSource_BilibiliImplCopyWith<_$QrSource_BilibiliImpl> get copyWith =>
      __$$QrSource_BilibiliImplCopyWithImpl<_$QrSource_BilibiliImpl>(
        this,
        _$identity,
      );

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(String field0) url,
    required TResult Function(String field0) image,
    required TResult Function(BigInt? waitSecs) screen,
    required TResult Function(String roomId, BigInt? waitSecs) bilibili,
  }) {
    return bilibili(roomId, waitSecs);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(String field0)? url,
    TResult? Function(String field0)? image,
    TResult? Function(BigInt? waitSecs)? screen,
    TResult? Function(String roomId, BigInt? waitSecs)? bilibili,
  }) {
    return bilibili?.call(roomId, waitSecs);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(String field0)? url,
    TResult Function(String field0)? image,
    TResult Function(BigInt? waitSecs)? screen,
    TResult Function(String roomId, BigInt? waitSecs)? bilibili,
    required TResult orElse(),
  }) {
    if (bilibili != null) {
      return bilibili(roomId, waitSecs);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(QrSource_Url value) url,
    required TResult Function(QrSource_Image value) image,
    required TResult Function(QrSource_Screen value) screen,
    required TResult Function(QrSource_Bilibili value) bilibili,
  }) {
    return bilibili(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(QrSource_Url value)? url,
    TResult? Function(QrSource_Image value)? image,
    TResult? Function(QrSource_Screen value)? screen,
    TResult? Function(QrSource_Bilibili value)? bilibili,
  }) {
    return bilibili?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(QrSource_Url value)? url,
    TResult Function(QrSource_Image value)? image,
    TResult Function(QrSource_Screen value)? screen,
    TResult Function(QrSource_Bilibili value)? bilibili,
    required TResult orElse(),
  }) {
    if (bilibili != null) {
      return bilibili(this);
    }
    return orElse();
  }
}

abstract class QrSource_Bilibili extends QrSource {
  const factory QrSource_Bilibili({required String roomId, BigInt? waitSecs}) =
      _$QrSource_BilibiliImpl;
  const QrSource_Bilibili._() : super._();

  String get roomId;
  BigInt? get waitSecs;

  /// Create a copy of QrSource
  /// with the given fields replaced by the non-null parameter values.
  @JsonKey(includeFromJson: false, includeToJson: false)
  _$$QrSource_BilibiliImplCopyWith<_$QrSource_BilibiliImpl> get copyWith =>
      throw _privateConstructorUsedError;
}
