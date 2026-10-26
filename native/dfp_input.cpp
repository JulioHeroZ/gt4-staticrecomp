#include "dfp_input.hpp"

#include <algorithm>
#include <cmath>

namespace gt4::dfp {

std::uint16_t DfpInputBridge::degrees_to_raw_14bit(float degrees, float max_degrees) noexcept {
  if (max_degrees <= 0.0f) {
    max_degrees = 900.0f;
  }
  const float half_lock = max_degrees * 0.5f;
  const float clamped_deg = std::clamp(degrees, -half_lock, half_lock);
  const float normalized = clamped_deg / half_lock; // -1.0 to +1.0

  const float raw_float = (normalized < 0.0f)
                              ? (static_cast<float>(kSteeringCenter) + (normalized * static_cast<float>(kSteeringCenter)))
                              : (static_cast<float>(kSteeringCenter) + (normalized * (static_cast<float>(kSteeringCenter) - 1.0f)));
  const auto raw_int = static_cast<std::int32_t>(std::round(raw_float));
  return static_cast<std::uint16_t>(std::clamp<std::int32_t>(raw_int, 0, static_cast<std::int32_t>(kSteeringMax)));
}

DfpInputState DfpInputBridge::encode_report(float normalized_steering,
                                             float normalized_throttle,
                                             float normalized_brake,
                                             std::uint16_t button_mask,
                                             std::uint8_t dpad_hat,
                                             bool connected) noexcept {
  DfpInputState state{};
  state.connected = connected;

  if (!connected) {
    state.steering = kSteeringCenter;
    return state;
  }

  // Steering: -1.0 .. +1.0 -> 0 .. 16383 (centered at 8192)
  const float clamped_steer = std::clamp(normalized_steering, -1.0f, 1.0f);
  const float raw_steer_f = (clamped_steer < 0.0f)
                                ? (static_cast<float>(kSteeringCenter) + (clamped_steer * static_cast<float>(kSteeringCenter)))
                                : (static_cast<float>(kSteeringCenter) + (clamped_steer * (static_cast<float>(kSteeringCenter) - 1.0f)));
  const auto raw_steer_i = static_cast<std::int32_t>(std::round(raw_steer_f));
  state.steering = static_cast<std::uint16_t>(std::clamp<std::int32_t>(raw_steer_i, 0, static_cast<std::int32_t>(kSteeringMax)));

  // Throttle: 0.0 .. 1.0 -> 0 .. 255
  const float clamped_throttle = std::clamp(normalized_throttle, 0.0f, 1.0f);
  state.throttle = static_cast<std::uint8_t>(std::clamp<std::int32_t>(
      static_cast<std::int32_t>(std::round(clamped_throttle * 255.0f)), 0, 255));

  // Brake: 0.0 .. 1.0 -> 0 .. 255
  const float clamped_brake = std::clamp(normalized_brake, 0.0f, 1.0f);
  state.brake = static_cast<std::uint8_t>(std::clamp<std::int32_t>(
      static_cast<std::int32_t>(std::round(clamped_brake * 255.0f)), 0, 255));

  state.buttons = button_mask;
  state.dpad = dpad_hat;
  return state;
}

void DfpInputBridge::update(DfpRpcServer &server,
                            float normalized_steering,
                            float normalized_throttle,
                            float normalized_brake,
                            std::uint16_t button_mask,
                            std::uint8_t dpad_hat,
                            bool connected) noexcept {
  const DfpInputState report = encode_report(normalized_steering,
                                             normalized_throttle,
                                             normalized_brake,
                                             button_mask,
                                             dpad_hat,
                                             connected);
  server.update_input(report);
}

} // namespace gt4::dfp
