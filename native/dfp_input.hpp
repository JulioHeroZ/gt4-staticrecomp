#pragma once

#include "dfp_rpc.hpp"
#include "wheel.hpp"

#include <cstdint>

namespace gt4::dfp {

// Encodes normalized host wheel controls into native 14-bit DFP guest reports
class DfpInputBridge {
public:
  DfpInputBridge() noexcept = default;

  // Convert host normalized inputs into DfpInputState
  // - normalized_steering: -1.0 (full left) to +1.0 (full right)
  // - normalized_throttle: 0.0 (released) to 1.0 (fully pressed)
  // - normalized_brake: 0.0 (released) to 1.0 (fully pressed)
  // - button_mask: PS2-mapped button bitmask
  // - dpad_hat: 0 = centered, 1 = up, 2 = right, 4 = down, 8 = left
  [[nodiscard]] static DfpInputState encode_report(float normalized_steering,
                                                   float normalized_throttle,
                                                   float normalized_brake,
                                                   std::uint16_t button_mask,
                                                   std::uint8_t dpad_hat,
                                                   bool connected = true) noexcept;

  // Connect and push current state to the SIF RPC server
  void update(DfpRpcServer &server,
              float normalized_steering,
              float normalized_throttle,
              float normalized_brake,
              std::uint16_t button_mask,
              std::uint8_t dpad_hat,
              bool connected = true) noexcept;

  // Steering angle mapping helper (degrees to 14-bit raw value)
  [[nodiscard]] static std::uint16_t degrees_to_raw_14bit(float degrees, float max_degrees = 900.0f) noexcept;
};

} // namespace gt4::dfp
