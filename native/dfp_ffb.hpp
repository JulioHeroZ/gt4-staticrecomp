#pragma once

#include "dfp_rpc.hpp"

#include <cstdint>
#include <string_view>

struct SDL_Haptic;

namespace gt4::dfp {

// Decoded Force Feedback Effect Types
enum class FfbEffectType : std::uint8_t {
  None = 0,
  ConstantForce,
  RampForce,
  Spring,
  Damper,
  Friction,
  StopAll,
  SetRange,
  Watchdog,
  Unknown
};

// Decoded High-Level Telemetry Command
struct DecodedFfbEffect {
  FfbEffectType type = FfbEffectType::None;
  float constant_level = 0.0f; // -1.0f (full left) to +1.0f (full right)
  float spring_strength = 0.0f; // 0.0f to 1.0f
  float spring_deadband = 0.0f; // 0.0f to 1.0f
  float damper_strength = 0.0f; // 0.0f to 1.0f
  float friction_strength = 0.0f; // 0.0f to 1.0f
  std::uint16_t range_degrees = 900u;
  bool stop_all = false;
};

// Force Feedback Engine: decodes guest GT4 physics packets and controls SDL3 Haptics
class DfpFfbEngine {
public:
  DfpFfbEngine() noexcept = default;
  ~DfpFfbEngine();

  DfpFfbEngine(const DfpFfbEngine &) = delete;
  DfpFfbEngine &operator=(const DfpFfbEngine &) = delete;

  // Initialize SDL3 haptics on target device (or mock mode when haptic is null)
  void attach(SDL_Haptic *haptic) noexcept;
  void detach() noexcept;
  [[nodiscard]] bool is_attached() const noexcept { return haptic_ != nullptr; }

  // Configuration
  void set_gain(float overall_gain) noexcept;
  [[nodiscard]] float gain() const noexcept { return gain_; }

  void set_spring_scale(float spring_scale) noexcept;
  [[nodiscard]] float spring_scale() const noexcept { return spring_scale_; }

  void set_damper_scale(float damper_scale) noexcept;
  [[nodiscard]] float damper_scale() const noexcept { return damper_scale_; }

  // Decode raw 8-byte guest packet
  [[nodiscard]] static DecodedFfbEffect decode_packet(const DfpFfbPacket &packet) noexcept;

  // Process incoming packet from SIF RPC server
  void process_packet(const DfpFfbPacket &packet) noexcept;

  // Drain pending packets from DfpRpcServer queue
  void drain_rpc_queue(DfpRpcServer &server) noexcept;

  // Fail-safe: stop all forces immediately (called on WM_KILLFOCUS, pause, or exit)
  void stop() noexcept;

  // Telemetry inspection for testing and UI display
  [[nodiscard]] const DecodedFfbEffect &last_effect() const noexcept { return last_effect_; }
  [[nodiscard]] std::uint64_t processed_packets_count() const noexcept { return packet_count_; }

private:
  SDL_Haptic *haptic_ = nullptr;
  float gain_ = 1.0f;
  float spring_scale_ = 1.0f;
  float damper_scale_ = 1.0f;

  DecodedFfbEffect last_effect_{};
  std::uint64_t packet_count_ = 0;

  // Internal effect slot IDs for SDL3
  int constant_effect_id_ = -1;
  int spring_effect_id_ = -1;
  int damper_effect_id_ = -1;

  void apply_constant(float level) noexcept;
  void apply_spring(float strength, float deadband) noexcept;
  void apply_damper(float strength) noexcept;
};

} // namespace gt4::dfp
