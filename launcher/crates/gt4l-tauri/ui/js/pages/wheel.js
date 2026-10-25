/*
 * Universal Racing Wheel Configurator (TASK-018).
 *
 * Mounted inside the Controls page when the "Racing Wheel" mode is active.
 * Communicates with Rust via:
 *   Commands:
 *     get_wheel_config()
 *     set_wheel_setting({key, value})
 *     apply_wheel_preset({preset})
 *     assign_wheel_binding({target, source, pedalMode})
 *     reset_wheel_binding({target, clear})
 *     reset_wheel_config()
 *     capture_wheel_binding({target})
 *     cancel_wheel_capture()
 *     test_wheel_ffb()
 *   Events:
 *     wheel-status
 *     wheel-live
 */
(function (root) {
  "use strict";
  var GT4 = (root.GT4 = root.GT4 || {});
  var h = GT4.ui.h;

  function call(command, args) {
    return GT4.backend.call(command, args);
  }

  function svgNode(tag, attrs, parent) {
    var node = document.createElementNS("http://www.w3.org/2000/svg", tag);
    Object.keys(attrs).forEach(function (key) {
      node.setAttribute(key, String(attrs[key]));
    });
    if (parent) {
      parent.appendChild(node);
    }
    return node;
  }

  var PRESET_LABELS = {
    generic: "Generic USB / DirectInput Wheel (900°)",
    logitech_g29: "Logitech G29 / G920 / G923 / G27 / G25 (900°)",
    logitech_dfgt: "Logitech Driving Force GT / Pro (900°)",
    thrustmaster: "Thrustmaster T150 / T248 / T300RS / TX / T-GT",
    fanatec_moza: "Fanatec / Moza / Simagic / Cammus Direct Drive",
    combined_pedals: "Combined Single-Axis Pedals (270°)"
  };

  var PEDAL_MAP_LABELS = {
    cross_square: "Default DS2 (Throttle → Cross, Brake → Square, Paddles → L2/R2)",
    both: "Dual (Throttle → Cross + R2, Brake → Square + L2)",
    l2_r2: "Triggers Only (Throttle → R2, Brake → L2)",
    rstick_y: "Right Stick Y Axis (Proportional Stick Gas/Brake)"
  };

  var PEDAL_MODE_LABELS = {
    inverted: "Inverted (+32767 rest → -32768 full, Logitech/Thrustmaster/HID)",
    normal: "Normal (-32768 rest → +32767 full, Direct Drive / Load Cell)",
    positive: "Unipolar Positive (0 rest → +32767 full)",
    combined_negative: "Combined Axis Negative Half (0 → -32768)",
    combined_positive: "Combined Axis Positive Half (0 → +32767)"
  };

  function formatSourceBadge(src) {
    if (!src) {
      return "Unbound";
    }
    if (src.indexOf("dev:") === 0) {
      var parts = src.split(":");
      var shortGuid = parts[1] ? parts[1].slice(0, 6) : "usb";
      var tail = parts.slice(2).join(":");
      return tail.toUpperCase() + " (USB " + shortGuid + ")";
    }
    return src.toUpperCase();
  }

  function evalSteeringNorm(cfg, raw) {
    var signed = raw < 0 ? raw / 32768.0 : raw / 32767.0;
    if (cfg.invert) {
      signed = -signed;
    }
    var sign = signed < 0 ? -1 : 1;
    var mag = Math.abs(signed);
    var phys = Math.max(180, Math.min(1080, cfg.rotation_degrees || 900));
    var eff = Math.max(180, Math.min(phys, cfg.effective_degrees || phys));
    var ratio = eff / phys;
    if (ratio > 0) {
      mag /= ratio;
    }
    var dz = Math.max(0, Math.min(0.45, cfg.deadzone || 0));
    if (mag <= dz) {
      return 0;
    }
    var norm = Math.max(0, Math.min(1, (mag - dz) / (1 - dz)));
    if (cfg.linearity === "quadratic") {
      norm = norm * norm;
    } else if (cfg.linearity === "cubic") {
      norm = norm * norm * norm;
    }
    var scaled = Math.max(0, Math.min(1, norm * (cfg.sensitivity || 1)));
    return sign * scaled;
  }

  function evalPedalNorm(cfg, raw) {
    var travel = 0;
    var mode = (cfg && cfg.mode) || "inverted";
    if (mode === "inverted") {
      travel = (32767 - raw) / 65535;
    } else if (mode === "normal") {
      travel = (raw + 32768) / 65535;
    } else if (mode === "positive" || mode === "combined_positive") {
      travel = raw <= 0 ? 0 : raw / 32767;
    } else if (mode === "combined_negative") {
      travel = raw >= 0 ? 0 : -raw / 32768;
    }
    travel = Math.max(0, Math.min(1, travel));
    var dz = Math.max(0, Math.min(0.45, (cfg && cfg.deadzone) || 0));
    var sat = Math.max(dz + 0.05, Math.min(1, (cfg && cfg.saturation) || 0.98));
    if (travel <= dz) {
      return 0;
    }
    var norm = Math.max(0, Math.min(1, (travel - dz) / (sat - dz)));
    if (cfg && cfg.curve === "quadratic") {
      norm = norm * norm;
    } else if (cfg && cfg.curve === "cubic") {
      norm = norm * norm * norm;
    }
    return Math.max(0, Math.min(1, norm));
  }

  function readSourceAxis(src, devices, primaryGuid) {
    if (!src || !devices || !devices.length) {
      return 0;
    }
    var devGuid = primaryGuid;
    var tail = src;
    if (src.indexOf("dev:") === 0) {
      var parts = src.split(":");
      devGuid = parts[1] || "";
      tail = parts.slice(2).join(":");
    }
    var dev = null;
    if (devGuid) {
      dev = devices.filter(function (d) {
        return (d.guid || "").toLowerCase() === devGuid.toLowerCase();
      })[0];
    }
    if (!dev) {
      dev =
        devices.filter(function (d) {
          return d.is_wheel;
        })[0] ||
        devices.filter(function (d) {
          return !d.is_gamepad;
        })[0] ||
        devices[0];
    }
    if (!dev || tail.indexOf("axis:") !== 0) {
      return 0;
    }
    var idx = parseInt(tail.slice(5), 10);
    return dev.axes && typeof dev.axes[idx] === "number" ? dev.axes[idx] : 0;
  }

  GT4.mountWheelConfigurator = function (container) {
    GT4.ui.clear(container);

    var state = {
      config: null,
      buttonSlots: [],
      devices: [],
      liveDevices: [],
      capturing: null
    };

    var rootEl = h("div", { class: "wh-root" });
    var bodyEl = h("div", { class: "wh-grid" });
    rootEl.appendChild(bodyEl);
    container.appendChild(rootEl);

    // Live SVG wheel & telemetry references:
    var wheelRimGroup = null;
    var steerDegReadout = null;
    var steerOutReadout = null;
    var activeBtnsReadout = null;
    var pedalBars = {};

    function applyView(view) {
      if (!view || !view.config) {
        return;
      }
      state.config = view.config;
      state.buttonSlots = view.button_slots || [];
      if (view.wheel_status && Array.isArray(view.wheel_status.devices)) {
        state.devices = view.wheel_status.devices;
      }
      render();
    }

    function load() {
      return call("get_wheel_config").then(applyView, function (err) {
        GT4.ui.toast(GT4.errorText(err), true);
      });
    }

    function updateSetting(key, value) {
      return call("set_wheel_setting", { key: key, value: value }).then(
        applyView,
        function (err) {
          GT4.ui.toast(GT4.errorText(err), true);
          load();
        }
      );
    }

    function startCapture(target, label) {
      if (state.capturing) {
        return;
      }
      var isSteering = target === "steering";
      var isPedal =
        target === "throttle" ||
        target === "brake" ||
        target === "handbrake" ||
        target === "clutch";
      var promptText = isSteering
        ? "Turn your steering wheel left or right past 30% travel…"
        : isPedal
        ? "Press and release the " + label + " pedal all the way down…"
        : "Press any wheel button, paddle, H-shifter gear, or D-pad direction for " +
          label +
          "…";

      var body = h("div", { class: "wh-capture-modal" }, [
        h("p", { class: "modal-text", text: promptText }),
        h("p", {
          class: "cx-capture-sub",
          text: "Inputs from standalone USB pedals, handbrakes, or H-shifters are detected automatically."
        })
      ]);

      var done = false;
      var dialog = null;

      function finish(reason, payload) {
        if (done) {
          return;
        }
        done = true;
        state.capturing = null;
        GT4.capturing = false;
        GT4.cancelCapture = null;
        call("cancel_wheel_capture").catch(function () {});
        if (dialog) {
          dialog.close();
        }
        if (reason === "captured" && payload && payload.source) {
          call("assign_wheel_binding", {
            target: target,
            source: payload.source,
            pedalMode: payload.detected_pedal_mode || null
          }).then(
            function (view) {
              applyView(view);
              var modeNote = payload.detected_pedal_mode
                ? " (" + payload.detected_pedal_mode + " axis)"
                : "";
              GT4.ui.toast(
                "Bound " + label + " to " + formatSourceBadge(payload.source) + modeNote
              );
            },
            function (err) {
              GT4.ui.toast(GT4.errorText(err), true);
            }
          );
        } else if (reason === "error" && payload) {
          GT4.ui.toast(GT4.errorText(payload), true);
        }
      }

      state.capturing = target;
      GT4.capturing = true;
      GT4.cancelCapture = function () {
        finish("cancel");
      };

      dialog = GT4.ui.openDialog({
        title: "Bind " + label,
        body: body,
        wide: false,
        dismissable: true,
        buttons: [
          {
            label: "Cancel",
            closes: false,
            run: function () {
              finish("cancel");
            }
          }
        ]
      });

      call("capture_wheel_binding", { target: target }).then(
        function (result) {
          finish("captured", result);
        },
        function (err) {
          if (!done && String(GT4.errorText(err)).indexOf("cancel") < 0) {
            finish("error", err);
          }
        }
      );
    }

    function makeSliderRow(label, current, min, max, step, unit, onChange) {
      var fmt = function (v) {
        if (step < 1) {
          return Number(v).toFixed(2) + (unit || "");
        }
        return String(Math.round(Number(v))) + (unit || "");
      };
      var valSpan = h("span", { class: "wh-slider-val", text: fmt(current) });
      var input = h("input", {
        type: "range",
        class: "slider",
        min: min,
        max: max,
        step: step,
        value: current,
        "aria-label": label
      });
      var updateFill = function () {
        var pct = ((Number(input.value) - min) / (max - min || 1)) * 100;
        input.style.setProperty("--fill", pct.toFixed(1) + "%");
        valSpan.textContent = fmt(input.value);
      };
      input.addEventListener("input", updateFill);
      input.addEventListener("change", function () {
        onChange(Number(input.value));
      });
      updateFill();
      return h("div", { class: "wh-field" }, [
        h("div", { class: "wh-field-head" }, [
          h("span", { class: "wh-field-label", text: label }),
          valSpan
        ]),
        input
      ]);
    }

    function makeSelectRow(label, current, options, onChange) {
      var select = h(
        "select",
        {
          class: "select wh-select",
          "aria-label": label,
          on: {
            change: function () {
              onChange(select.value);
            }
          }
        },
        options.map(function (opt) {
          return h(
            "option",
            { value: opt.value, selected: opt.value === current },
            opt.label
          );
        })
      );
      return h("div", { class: "wh-field" }, [
        h("div", { class: "wh-field-head" }, [
          h("span", { class: "wh-field-label", text: label })
        ]),
        select
      ]);
    }

    function makeToggleRow(label, checked, onChange) {
      var btn = h(
        "button",
        {
          type: "button",
          class: "btn small " + (checked ? "primary" : ""),
          "aria-pressed": String(!!checked),
          on: {
            click: function () {
              onChange(!checked);
            }
          }
        },
        checked ? "Enabled" : "Disabled"
      );
      return h("div", { class: "wh-field wh-field-inline" }, [
        h("span", { class: "wh-field-label", text: label }),
        btn
      ]);
    }

    function buildTelemetryCard(cfg) {
      var svg = svgNode("svg", {
        viewBox: "-60 -60 120 120",
        class: "wh-wheel-svg",
        "aria-hidden": "true"
      });
      svgNode(
        "circle",
        { cx: 0, cy: 0, r: 52, class: "wh-wheel-track" },
        svg
      );
      wheelRimGroup = svgNode("g", { class: "wh-wheel-rim" }, svg);
      svgNode("circle", { cx: 0, cy: 0, r: 44, class: "wh-wheel-outer" }, wheelRimGroup);
      svgNode(
        "path",
        { d: "M -42 0 L 42 0 M 0 0 L 0 42", class: "wh-wheel-spokes" },
        wheelRimGroup
      );
      svgNode("circle", { cx: 0, cy: 0, r: 11, class: "wh-wheel-hub" }, wheelRimGroup);
      svgNode(
        "rect",
        { x: -4, y: -48, width: 8, height: 9, rx: 2, class: "wh-wheel-marker" },
        wheelRimGroup
      );

      steerDegReadout = h("div", { class: "wh-steer-deg", text: "0°" });
      steerOutReadout = h("div", {
        class: "wh-steer-sub",
        text: "PS2 Steering: 128 (Center)"
      });

      pedalBars = {};
      var pedalMetersNode = h(
        "div",
        { class: "wh-pedal-meters" },
        [
          ["throttle", "Throttle"],
          ["brake", "Brake"],
          ["handbrake", "Handbrake"],
          ["clutch", "Clutch"]
        ].map(function (pair) {
          var fill = h("div", { class: "wh-pedal-fill is-" + pair[0] });
          var pct = h("span", { class: "wh-pedal-pct", text: "0%" });
          pedalBars[pair[0]] = { fill: fill, pct: pct };
          return h("div", { class: "wh-pedal-col" }, [
            h("span", { class: "wh-pedal-name", text: pair[1] }),
            h("div", { class: "wh-pedal-bar" }, [fill]),
            pct
          ]);
        })
      );

      activeBtnsReadout = h("div", {
        class: "wh-live-btns",
        text: "Active inputs: None"
      });

      var deviceCount = state.devices.length;
      var statusBadge = h(
        "span",
        { class: "wh-status-pill " + (deviceCount > 0 ? "is-on" : "") },
        deviceCount > 0
          ? deviceCount + " USB device(s) detected"
          : "No USB wheel detected yet"
      );

      return h("section", { class: "wh-card wh-card-telemetry" }, [
        h("div", { class: "wh-card-head" }, [
          h("h2", { text: "Live Wheel & Pedal Telemetry" }),
          statusBadge
        ]),
        h("div", { class: "wh-telemetry-body" }, [
          h("div", { class: "wh-wheel-box" }, [
            svg,
            steerDegReadout,
            steerOutReadout
          ]),
          pedalMetersNode
        ]),
        activeBtnsReadout,
        h("p", {
          class: "wh-note",
          text:
            "Lock: " +
            cfg.steering.rotation_degrees +
            "° physical / " +
            cfg.steering.effective_degrees +
            "° effective"
        })
      ]);
    }

    function buildDeviceAndPresetCard(cfg) {
      var deviceOptions = [
        {
          value: "",
          label: "Auto-detect Primary Racing Wheel (Recommended)"
        }
      ];
      state.devices.forEach(function (d) {
        var tag = d.is_wheel ? "Wheel" : d.is_gamepad ? "Gamepad" : "USB HID";
        var ffbTag = d.has_ffb ? " · FFB" : "";
        deviceOptions.push({
          value: d.guid,
          label:
            d.name +
            " [" +
            tag +
            " · " +
            d.num_axes +
            " axes, " +
            d.num_buttons +
            " btns" +
            ffbTag +
            "]"
        });
      });
      if (
        cfg.device_guid &&
        !deviceOptions.some(function (o) {
          return o.value === cfg.device_guid;
        })
      ) {
        deviceOptions.push({
          value: cfg.device_guid,
          label:
            (cfg.device_name || "Saved Wheel") +
            " (" +
            cfg.device_guid.slice(0, 8) +
            "… · disconnected)"
        });
      }

      var presetOptions = Object.keys(PRESET_LABELS).map(function (k) {
        return { value: k, label: PRESET_LABELS[k] };
      });
      if (cfg.preset === "custom") {
        presetOptions.push({ value: "custom", label: "Custom Configuration" });
      }

      var pedalMapOptions = Object.keys(PEDAL_MAP_LABELS).map(function (k) {
        return { value: k, label: PEDAL_MAP_LABELS[k] };
      });

      return h("section", { class: "wh-card" }, [
        h("div", { class: "wh-card-head" }, [
          h("h2", { text: "Wheel Device & Profile" }),
          h(
            "button",
            {
              type: "button",
              class: "btn danger small",
              on: {
                click: function () {
                  call("reset_wheel_config").then(applyView, function (err) {
                    GT4.ui.toast(GT4.errorText(err), true);
                  });
                }
              }
            },
            "Reset All Wheel Settings"
          )
        ]),
        makeToggleRow("Enable Racing Wheel Input", cfg.enabled, function (val) {
          updateSetting("wheel.enabled", val);
        }),
        makeSelectRow(
          "Primary Racing Wheel Device",
          cfg.device_guid || "",
          deviceOptions,
          function (guid) {
            var chosen = state.devices.filter(function (d) {
              return d.guid === guid;
            })[0];
            updateSetting("wheel.device_guid", guid).then(function () {
              if (chosen) {
                updateSetting("wheel.device_name", chosen.name);
              }
            });
          }
        ),
        makeSelectRow(
          "Hardware Preset (Logitech, Thrustmaster, Direct Drive, Generic)",
          cfg.preset || "generic",
          presetOptions,
          function (preset) {
            if (preset !== "custom") {
              call("apply_wheel_preset", { preset: preset }).then(
                applyView,
                function (err) {
                  GT4.ui.toast(GT4.errorText(err), true);
                }
              );
            }
          }
        ),
        makeSelectRow(
          "In-Game Analog Pedal Target",
          cfg.pedal_map_mode || "cross_square",
          pedalMapOptions,
          function (mode) {
            updateSetting("wheel.pedal_map_mode", mode);
          }
        )
      ]);
    }

    function buildSteeringCard(cfg) {
      var st = cfg.steering;
      var degreePresets = [200, 270, 360, 540, 720, 900, 1080];
      var quickDegrees = h(
        "div",
        { class: "wh-deg-pills" },
        degreePresets.map(function (deg) {
          return h(
            "button",
            {
              type: "button",
              class:
                "btn small " +
                (st.rotation_degrees === deg && st.effective_degrees === deg
                  ? "primary"
                  : ""),
              on: {
                click: function () {
                  updateSetting("steering.rotation_degrees", deg).then(
                    function () {
                      updateSetting("steering.effective_degrees", deg);
                    }
                  );
                }
              }
            },
            deg + "°"
          );
        })
      );

      return h("section", { class: "wh-card" }, [
        h("div", { class: "wh-card-head" }, [
          h("h2", { text: "Steering Axis & Rotation Lock" }),
          h("div", { class: "wh-inline-btns" }, [
            h("span", {
              class: "xchip kind-trigger",
              text: formatSourceBadge(st.source)
            }),
            h(
              "button",
              {
                type: "button",
                class: "btn primary small",
                on: {
                  click: function () {
                    startCapture("steering", "Steering Wheel Axis");
                  }
                }
              },
              "Bind Steering"
            ),
            h(
              "button",
              {
                type: "button",
                class: "btn small",
                on: {
                  click: function () {
                    call("reset_wheel_binding", {
                      target: "steering",
                      clear: false
                    }).then(applyView);
                  }
                }
              },
              "Default"
            )
          ])
        ]),
        quickDegrees,
        makeSliderRow(
          "Physical Wheel Rotation (Lock-to-Lock)",
          st.rotation_degrees,
          180,
          1080,
          10,
          "°",
          function (deg) {
            updateSetting("steering.rotation_degrees", Math.round(deg));
          }
        ),
        makeSliderRow(
          "Effective In-Game Steering Lock",
          st.effective_degrees,
          180,
          1080,
          10,
          "°",
          function (deg) {
            updateSetting("steering.effective_degrees", Math.round(deg));
          }
        ),
        makeSliderRow(
          "Steering Center Deadzone",
          Math.round(st.deadzone * 100),
          0,
          45,
          1,
          "%",
          function (pct) {
            updateSetting("steering.deadzone", pct / 100);
          }
        ),
        makeSelectRow(
          "Steering Linearity Curve",
          st.linearity,
          [
            { value: "linear", label: "Linear (1:1 Direct Proportional)" },
            { value: "quadratic", label: "Quadratic (Softer Near Center)" },
            { value: "cubic", label: "Cubic (Fine Center Precision)" }
          ],
          function (curve) {
            updateSetting("steering.linearity", curve);
          }
        ),
        makeSliderRow(
          "Steering Sensitivity",
          st.sensitivity,
          0.25,
          2.5,
          0.05,
          "x",
          function (sens) {
            updateSetting("steering.sensitivity", sens);
          }
        ),
        makeToggleRow("Invert Steering Axis", st.invert, function (inv) {
          updateSetting("steering.invert", inv);
        })
      ]);
    }

    function buildPedalsCard(cfg) {
      var modeOptions = Object.keys(PEDAL_MODE_LABELS).map(function (k) {
        return { value: k, label: PEDAL_MODE_LABELS[k] };
      });
      var curveOptions = [
        { value: "linear", label: "Linear" },
        { value: "quadratic", label: "Quadratic (Progressive)" },
        { value: "cubic", label: "Cubic" }
      ];

      function pedalSection(id, title, pedal) {
        return h("div", { class: "wh-pedal-config" }, [
          h("div", { class: "wh-card-head" }, [
            h("h3", { text: title }),
            h("div", { class: "wh-inline-btns" }, [
              h("span", {
                class: "xchip kind-trigger",
                text: formatSourceBadge(pedal.source)
              }),
              h(
                "button",
                {
                  type: "button",
                  class: "btn primary small",
                  on: {
                    click: function () {
                      startCapture(id, title);
                    }
                  }
                },
                "Bind " + title
              ),
              h(
                "button",
                {
                  type: "button",
                  class: "btn small",
                  on: {
                    click: function () {
                      call("reset_wheel_binding", {
                        target: id,
                        clear: true
                      }).then(applyView);
                    }
                  }
                },
                "Unbind"
              )
            ])
          ]),
          makeSelectRow("Axis Polarity Mode", pedal.mode, modeOptions, function (
            m
          ) {
            updateSetting(id + ".mode", m);
          }),
          makeSliderRow(
            "Deadzone",
            Math.round(pedal.deadzone * 100),
            0,
            45,
            1,
            "%",
            function (pct) {
              updateSetting(id + ".deadzone", pct / 100);
            }
          ),
          makeSliderRow(
            "Saturation (Full Travel Point)",
            Math.round(pedal.saturation * 100),
            20,
            100,
            1,
            "%",
            function (pct) {
              updateSetting(id + ".saturation", pct / 100);
            }
          ),
          makeSelectRow("Response Curve", pedal.curve, curveOptions, function (
            c
          ) {
            updateSetting(id + ".curve", c);
          })
        ]);
      }

      return h("section", { class: "wh-card" }, [
        h("div", { class: "wh-card-head" }, [
          h("h2", { text: "Pedals & Handbrake Calibration" })
        ]),
        pedalSection("throttle", "Throttle Pedal", cfg.throttle),
        pedalSection("brake", "Brake Pedal", cfg.brake),
        pedalSection("handbrake", "Handbrake Axis", cfg.handbrake),
        pedalSection("clutch", "Clutch Pedal", cfg.clutch)
      ]);
    }

    function buildFfbCard(cfg) {
      var ffb = cfg.ffb;
      return h("section", { class: "wh-card" }, [
        h("div", { class: "wh-card-head" }, [
          h("h2", { text: "Force Feedback & Haptics" }),
          h(
            "button",
            {
              type: "button",
              class: "btn primary small",
              on: {
                click: function () {
                  call("test_wheel_ffb").then(
                    function (res) {
                      GT4.ui.toast(
                        "Force Feedback test pulse sent to " +
                          ((res && res.device_name) || "wheel") +
                          "."
                      );
                    },
                    function (err) {
                      GT4.ui.toast(GT4.errorText(err), true);
                    }
                  );
                }
              }
            },
            "Test Force Feedback"
          )
        ]),
        makeToggleRow("Enable Force Feedback", ffb.enabled, function (val) {
          updateSetting("ffb.enabled", val);
        }),
        makeSliderRow(
          "Overall Force Strength",
          Math.round(ffb.strength * 100),
          0,
          200,
          5,
          "%",
          function (pct) {
            updateSetting("ffb.strength", pct / 100);
          }
        ),
        makeSliderRow(
          "Centering Spring Force",
          Math.round(ffb.centering_spring * 100),
          0,
          100,
          5,
          "%",
          function (pct) {
            updateSetting("ffb.centering_spring", pct / 100);
          }
        ),
        makeSliderRow(
          "Wheel Damper / Friction",
          Math.round(ffb.damper * 100),
          0,
          100,
          5,
          "%",
          function (pct) {
            updateSetting("ffb.damper", pct / 100);
          }
        ),
        makeSliderRow(
          "Road & Curb Rumble Gain",
          Math.round(ffb.curb_rumble * 100),
          0,
          200,
          5,
          "%",
          function (pct) {
            updateSetting("ffb.curb_rumble", pct / 100);
          }
        ),
        makeToggleRow("Invert Force Feedback Direction", ffb.invert, function (
          inv
        ) {
          updateSetting("ffb.invert", inv);
        })
      ]);
    }

    function buildButtonsCard() {
      var rows = state.buttonSlots.map(function (slot) {
        var badges = (slot.bindings || []).length
          ? slot.bindings.map(function (src) {
              return h("span", {
                class: "xchip kind-bumper",
                text: formatSourceBadge(src)
              });
            })
          : [h("span", { class: "xchip kind-none", text: "Unbound" })];

        return h("div", { class: "wh-btn-row" }, [
          h("span", { class: "wh-btn-label", text: slot.label }),
          h("div", { class: "wh-btn-chips" }, badges),
          h("div", { class: "wh-inline-btns" }, [
            h(
              "button",
              {
                type: "button",
                class: "btn primary small",
                on: {
                  click: function () {
                    startCapture(slot.id, slot.label);
                  }
                }
              },
              "Bind"
            ),
            h(
              "button",
              {
                type: "button",
                class: "btn small",
                on: {
                  click: function () {
                    call("reset_wheel_binding", {
                      target: slot.id,
                      clear: false
                    }).then(applyView);
                  }
                }
              },
              "Reset"
            ),
            h(
              "button",
              {
                type: "button",
                class: "btn small",
                on: {
                  click: function () {
                    call("reset_wheel_binding", {
                      target: slot.id,
                      clear: true
                    }).then(applyView);
                  }
                }
              },
              "Clear"
            )
          ])
        ]);
      });

      return h("section", { class: "wh-card wh-card-wide" }, [
        h("div", { class: "wh-card-head" }, [
          h("h2", {
            text: "Paddles, Shifter, D-Pad & Wheel Buttons (All 16 PS2 Inputs)"
          })
        ]),
        h("p", {
          class: "wh-note",
          text:
            "Up to 3 physical buttons, paddles, hat directions, or external USB shifter inputs can be bound to each action."
        }),
        h("div", { class: "wh-btn-list" }, rows)
      ]);
    }

    function render() {
      if (!state.config) {
        return;
      }
      GT4.ui.clear(bodyEl);
      bodyEl.appendChild(buildDeviceAndPresetCard(state.config));
      bodyEl.appendChild(buildTelemetryCard(state.config));
      bodyEl.appendChild(buildSteeringCard(state.config));
      bodyEl.appendChild(buildFfbCard(state.config));
      bodyEl.appendChild(buildPedalsCard(state.config));
      bodyEl.appendChild(buildButtonsCard());
      paintLiveTelemetry();
    }

    function paintLiveTelemetry() {
      if (!state.config || !wheelRimGroup) {
        return;
      }
      var cfg = state.config;
      var devices = state.liveDevices || [];
      var rawSteer = readSourceAxis(
        cfg.steering.source,
        devices,
        cfg.device_guid
      );
      var normSteer = evalSteeringNorm(cfg.steering, rawSteer);
      var maxHalfDeg = (cfg.steering.rotation_degrees || 900) / 2;
      var currentDeg = Math.round(normSteer * maxHalfDeg);
      var ps2Byte =
        normSteer < 0
          ? Math.max(0, Math.round(128 + normSteer * 128))
          : Math.min(255, Math.round(128 + normSteer * 127));

      wheelRimGroup.setAttribute("transform", "rotate(" + currentDeg + ")");
      if (steerDegReadout) {
        steerDegReadout.textContent =
          (currentDeg > 0 ? "+" : "") + currentDeg + "°";
      }
      if (steerOutReadout) {
        steerOutReadout.textContent = "PS2 Steering Byte: " + ps2Byte;
      }

      ["throttle", "brake", "handbrake", "clutch"].forEach(function (id) {
        var pCfg = cfg[id];
        var bar = pedalBars[id];
        if (!pCfg || !bar) {
          return;
        }
        var raw = readSourceAxis(pCfg.source, devices, cfg.device_guid);
        var val = pCfg.source ? evalPedalNorm(pCfg, raw) : 0;
        var pct = Math.round(val * 100);
        bar.fill.style.height = pct + "%";
        bar.pct.textContent = pct + "%";
      });

      if (activeBtnsReadout) {
        var pressedList = [];
        devices.forEach(function (d) {
          (d.buttons || []).forEach(function (down, idx) {
            if (down) {
              pressedList.push(d.name + " BTN:" + idx);
            }
          });
          (d.hats || []).forEach(function (mask, idx) {
            if (mask & 1) pressedList.push("HAT:" + idx + ":UP");
            if (mask & 2) pressedList.push("HAT:" + idx + ":RIGHT");
            if (mask & 4) pressedList.push("HAT:" + idx + ":DOWN");
            if (mask & 8) pressedList.push("HAT:" + idx + ":LEFT");
          });
        });
        activeBtnsReadout.textContent = pressedList.length
          ? "Active inputs: " + pressedList.join(" · ")
          : "Active inputs: None";
      }
    }

    GT4.backend.listen("wheel-status", function (payload) {
      if (!rootEl.isConnected) {
        return;
      }
      if (payload && Array.isArray(payload.devices)) {
        state.devices = payload.devices;
        render();
      }
    });

    GT4.backend.listen("wheel-live", function (payload) {
      if (!rootEl.isConnected) {
        return;
      }
      if (payload && Array.isArray(payload.devices)) {
        state.liveDevices = payload.devices;
        paintLiveTelemetry();
      }
    });

    load();
  };
})(window);
