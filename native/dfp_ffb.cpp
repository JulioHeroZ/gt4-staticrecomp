#include "dfp_ffb.hpp"

#include <SDL3/SDL.h>

#include <algorithm>
#include <cmath>
#include <cstring>

namespace gt4::dfp {

DfpFfbEngine::~DfpFfbEngine() {
  stop();
  detach();
}

void DfpFfbEngine::attach(SDL_Haptic *haptic) noexcept {
  if (haptic_ == haptic)
    return;

  stop();
  detach();
  haptic_ = haptic;
}

void DfpFfbEngine::detach() noexcept {
  stop();
  haptic_ = nullptr;
}

void DfpFfbEngine::set_gain(float overall_gain) noexcept {
  gain_ = std::clamp(overall_gain, 0.0f, 2.0f);
}

void DfpFfbEngine::set_spring_scale(float spring_scale) noexcept {
  spring_scale_ = std::clamp(spring_scale, 0.0f, 2.0f);
}

void DfpFfbEngine::set_damper_scale(float damper_scale) noexcept {
  damper_scale_ = std::clamp(damper_scale, 0.0f, 2.0f);
}

DecodedFfbEffect DfpFfbEngine::decode_packet(const DfpFfbPacket &packet) noexcept {
  DecodedFfbEffect effect{};

  switch (packet.command()) {
  case 0x00: {
    // Constant force: Byte 1 is signed 8-bit force (-128 to 127)
    effect.type = FfbEffectType::ConstantForce;
    const auto raw_level = static_cast<std::int8_t>(packet.data[1]);
    effect.constant_level = std::clamp(static_cast<float>(raw_level) / 127.0f, -1.0f, 1.0f);
    break;
  }

  case 0x01: {
    // Ramp force
    effect.type = FfbEffectType::RampForce;
    const auto start_val = static_cast<std::int8_t>(packet.data[1]);
    effect.constant_level = std::clamp(static_cast<float>(start_val) / 127.0f, -1.0f, 1.0f);
    break;
  }

  case 0x02: {
    // Auto-centering spring
    effect.type = FfbEffectType::Spring;
    effect.spring_deadband = static_cast<float>(packet.data[1]) / 255.0f;
    const float k_left = static_cast<float>(packet.data[2]) / 255.0f;
    const float k_right = static_cast<float>(packet.data[3]) / 255.0f;
    effect.spring_strength = std::max(k_left, k_right);
    break;
  }

  case 0x03: {
    // Damper & friction
    effect.type = FfbEffectType::Damper;
    effect.damper_strength = static_cast<float>(packet.data[1]) / 255.0f;
    effect.friction_strength = static_cast<float>(packet.data[2]) / 255.0f;
    break;
  }

  case 0x08:
  case 0xF8: {
    // Set rotation range
    effect.type = FfbEffectType::SetRange;
    const std::uint16_t range = static_cast<std::uint16_t>(packet.data[1]) |
                                (static_cast<std::uint16_t>(packet.data[2]) << 8);
    effect.range_degrees = (range > 0) ? range : 900u;
    break;
  }

  case 0x0B: {
    // Stop all effects / reset
    effect.type = FfbEffectType::StopAll;
    effect.stop_all = true;
    break;
  }

  case 0x0E: {
    // Watchdog
    effect.type = FfbEffectType::Watchdog;
    break;
  }

  default:
    effect.type = FfbEffectType::Unknown;
    break;
  }

  return effect;
}

void DfpFfbEngine::apply_constant(float level) noexcept {
  if (!haptic_)
    return;

  const float scaled = std::clamp(level * gain_, -1.0f, 1.0f);
  const auto sdl_level = static_cast<Sint16>(std::round(scaled * 32767.0f));

  if (constant_effect_id_ < 0) {
    SDL_HapticEffect effect{};
    effect.type = SDL_HAPTIC_CONSTANT;
    effect.constant.direction.type = SDL_HAPTIC_POLAR;
    effect.constant.direction.dir[0] = 0;
    effect.constant.length = SDL_HAPTIC_INFINITY;
    effect.constant.level = sdl_level;
    constant_effect_id_ = SDL_CreateHapticEffect(haptic_, &effect);
    if (constant_effect_id_ >= 0) {
      SDL_RunHapticEffect(haptic_, constant_effect_id_, 1);
    }
  } else {
    SDL_HapticEffect effect{};
    effect.type = SDL_HAPTIC_CONSTANT;
    effect.constant.direction.type = SDL_HAPTIC_POLAR;
    effect.constant.direction.dir[0] = 0;
    effect.constant.length = SDL_HAPTIC_INFINITY;
    effect.constant.level = sdl_level;
    SDL_UpdateHapticEffect(haptic_, constant_effect_id_, &effect);
  }
}

void DfpFfbEngine::apply_spring(float strength, float deadband) noexcept {
  if (!haptic_)
    return;

  const float scaled_strength = std::clamp(strength * gain_ * spring_scale_, 0.0f, 1.0f);
  const auto sdl_coeff = static_cast<Sint16>(std::round(scaled_strength * 32767.0f));
  const auto sdl_deadband = static_cast<Uint16>(std::round(deadband * 32767.0f));

  if (spring_effect_id_ < 0) {
    SDL_HapticEffect effect{};
    effect.type = SDL_HAPTIC_SPRING;
    effect.condition.length = SDL_HAPTIC_INFINITY;
    effect.condition.right_coeff[0] = sdl_coeff;
    effect.condition.left_coeff[0] = sdl_coeff;
    effect.condition.right_sat[0] = 0x7FFF;
    effect.condition.left_sat[0] = 0x7FFF;
    effect.condition.deadband[0] = sdl_deadband;
    spring_effect_id_ = SDL_CreateHapticEffect(haptic_, &effect);
    if (spring_effect_id_ >= 0) {
      SDL_RunHapticEffect(haptic_, spring_effect_id_, 1);
    }
  } else {
    SDL_HapticEffect effect{};
    effect.type = SDL_HAPTIC_SPRING;
    effect.condition.length = SDL_HAPTIC_INFINITY;
    effect.condition.right_coeff[0] = sdl_coeff;
    effect.condition.left_coeff[0] = sdl_coeff;
    effect.condition.right_sat[0] = 0x7FFF;
    effect.condition.left_sat[0] = 0x7FFF;
    effect.condition.deadband[0] = sdl_deadband;
    SDL_UpdateHapticEffect(haptic_, spring_effect_id_, &effect);
  }
}

void DfpFfbEngine::apply_damper(float strength) noexcept {
  if (!haptic_)
    return;

  const float scaled = std::clamp(strength * gain_ * damper_scale_, 0.0f, 1.0f);
  const auto sdl_coeff = static_cast<Sint16>(std::round(scaled * 32767.0f));

  if (damper_effect_id_ < 0) {
    SDL_HapticEffect effect{};
    effect.type = SDL_HAPTIC_DAMPER;
    effect.condition.length = SDL_HAPTIC_INFINITY;
    effect.condition.right_coeff[0] = sdl_coeff;
    effect.condition.left_coeff[0] = sdl_coeff;
    damper_effect_id_ = SDL_CreateHapticEffect(haptic_, &effect);
    if (damper_effect_id_ >= 0) {
      SDL_RunHapticEffect(haptic_, damper_effect_id_, 1);
    }
  } else {
    SDL_HapticEffect effect{};
    effect.type = SDL_HAPTIC_DAMPER;
    effect.condition.length = SDL_HAPTIC_INFINITY;
    effect.condition.right_coeff[0] = sdl_coeff;
    effect.condition.left_coeff[0] = sdl_coeff;
    SDL_UpdateHapticEffect(haptic_, damper_effect_id_, &effect);
  }
}

void DfpFfbEngine::process_packet(const DfpFfbPacket &packet) noexcept {
  ++packet_count_;
  const DecodedFfbEffect effect = decode_packet(packet);
  last_effect_ = effect;

  switch (effect.type) {
  case FfbEffectType::ConstantForce:
  case FfbEffectType::RampForce:
    apply_constant(effect.constant_level);
    break;

  case FfbEffectType::Spring:
    apply_spring(effect.spring_strength, effect.spring_deadband);
    break;

  case FfbEffectType::Damper:
    apply_damper(effect.damper_strength);
    break;

  case FfbEffectType::StopAll:
    stop();
    break;

  default:
    break;
  }
}

void DfpFfbEngine::drain_rpc_queue(DfpRpcServer &server) noexcept {
  DfpFfbPacket packet{};
  while (server.poll_ffb(packet)) {
    process_packet(packet);
  }
}

void DfpFfbEngine::stop() noexcept {
  if (haptic_) {
    SDL_StopHapticEffects(haptic_);
    if (constant_effect_id_ >= 0) {
      SDL_DestroyHapticEffect(haptic_, constant_effect_id_);
      constant_effect_id_ = -1;
    }
    if (spring_effect_id_ >= 0) {
      SDL_DestroyHapticEffect(haptic_, spring_effect_id_);
      spring_effect_id_ = -1;
    }
    if (damper_effect_id_ >= 0) {
      SDL_DestroyHapticEffect(haptic_, damper_effect_id_);
      damper_effect_id_ = -1;
    }
  }
  last_effect_.constant_level = 0.0f;
  last_effect_.spring_strength = 0.0f;
  last_effect_.damper_strength = 0.0f;
  last_effect_.stop_all = true;
}

} // namespace gt4::dfp
