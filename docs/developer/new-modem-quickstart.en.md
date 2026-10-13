# New Modem Quickstart: from VID/PID to merged

> 中文: [new-modem-quickstart.zh-cn.md](new-modem-quickstart.zh-cn.md)

This is the shortest checklist from "a new modem in hand (USB VID/PID known)"
to "code merged". For background and the legacy walkthrough see
[How_to_add_modem_cn.md](../How_to_add_modem_cn.md); architecture and layering
rules are in [modem-adaptation.zh-cn.md](modem-adaptation.zh-cn.md); Phase 4
decisions are recorded in [ADR-0001](../adr/0001-refactor-phase4-decisions.zh-cn.md);
the rollout log of the template itself is
[journal-newmodem-tpl](../refactor/journal-newmodem-tpl.md).

## Steps at a glance

| Step | Output | Automation | Verification |
| --- | --- | --- | --- |
| ① Register | modem_support.json entry | manual edit + `jq` | `jq` + `check_modem_registry.py` |
| ② Collect | recognition fixture | `qmodem_collect` | `qmodem_collect status` |
| ③ Bootstrap | cmds/vendor scripts | `cp vendor_template.sh` | `sh -n` |
| ④ Replay | parsing layer passing replay | fixture replay runners | `test_*_fixtures.sh` / `run_tests.sh` |
| ⑤ Table | support_list.md entry | `update_support_list.py` | `check_modem_registry.py` |
| ⑥ Submit | PR | git + CI | green CI |

## ① Register the modem_support.json entry

VID/PID is the starting point: the USB VID maps to `manufacturer_id`
(lowercase hex, e.g. `1e0e`); the PID distinguishes entries under the same
VID (e.g. `air724ug`). Add an entry under `modem_support.usb` in
`application/qmodem/files/usr/share/qmodem/modem_support.json`:

```json
"my-model": {
    "manufacturer_id": "1e0e",
    "manufacturer": "simcom",
    "platform": "qualcomm",
    "data_interface": "usb",
    "pdp_index": "1",
    "wcdma_band": "0",
    "lte_band": "1/3/5/8",
    "nsa_band": "0",
    "sa_band": "0",
    "modes": ["ecm", "rndis"]
}
```

Fields: `manufacturer` must match the vendor name in
`vendor/dynamic_load.json` and the `cmds/<vendor>.sh` file name;
`platform` (qualcomm/mediatek/unisoc/...) is the second path level of the
fixture directory; `modes` is the dial-mode whitelist (qmi/ecm/mbim/rndis).
The UCI `name` (Modem Model) equals the entry key.

Verify:

```sh
jq -e '.modem_support.usb["my-model"]' \
  application/qmodem/files/usr/share/qmodem/modem_support.json
python3 scripts/check_modem_registry.py   # steps ②⑤ pending; fixture hint allowed
```

## ② Collect the recognition fixture (real hardware, once)

Before `cmds/<vendor>.sh` exists, the recognition flow sends probe commands
into `recognition/pending/`. On-device collection (see
[testcases/README.md](../../testcases/README.md)):

```sh
uci set qmodem.main.testcase_collect=1 && uci commit qmodem
# replug / rescan to run recognition; trigger base_info etc. via LuCI/ubus
qmodem_collect status
qmodem_collect pack        # sanitized + encrypted, prints a review key
```

On the development machine:

```sh
scp root@<device>:/tmp/qmodem_feedback_*.tar .
# after the maintainer runs qmodem-seal decrypt:
scripts/import_testcases.sh qmodem_testcases_*.tar.gz
```

The collector archives records under
`testcases/recognition/<vendor>/<platform>/<model>/` with an
`expected_identity` field; unattributable pending records are rejected by
the import script. Check the result:

```sh
ls testcases/recognition/<vendor>/<platform>/<model>/
jq -e '.expected_identity' testcases/recognition/*/*/*/ATI-*.json
```

## ③ Bootstrap from the template

```sh
cd application/qmodem/files/usr/share/qmodem
cp cmds/vendor_template.sh cmds/<vendor>.sh
cp cmds/vendor_template.sh vendor/<vendor>.sh   # or split into two layers
```

Every section of the template carries a "what to fill here" comment:
identity probing (`cmd_<vendor>_identity_query`),
`get_imei`/`sim_info`/`base_info`/`network_info` (parsing must use the
`ql_at_clean`/`ql_csv_field`/`ql_regex_pick` patterns from
`cmds/qmodem_lib.sh`), optional band/net-lock (`get_lockband`/
`set_network_prefer`; declare unsupported features in
`vendor_get_disabled_features`), and the dial integration points
(`get_mode`/`set_mode`, consistent with `modes` from step ①).
Replace `myvendor`/placeholders with your vendor name.

Verify:

```sh
sh -n cmds/<vendor>.sh && sh -n vendor/<vendor>.sh
```

Adding a model to an existing vendor skips this step — do ① and extend the
existing `cmds/<vendor>.sh` if needed.

## ④ Replay fixtures (no hardware needed)

```sh
# recognition replay + vendor read-only JSON validity + expected golden compare
bash application/qmodem/tests/test_recognition_fixtures.sh
bash application/qmodem/tests/test_vendor_fixtures.sh
# repo-wide shell regression (includes qmodem_lib edge cases)
bash tests/sh/run_tests.sh
```

Three checks: path matches metadata, per-device-profile replay of
`at`/`fastat`, and exact compare against `expected/<method>.json`. To add a
golden output for a parser, drop `<method>.json` into
`testcases/<vendor>/<platform>/<model>-<md5-prefix>/expected/`. See
[testing.zh-cn.md](testing.zh-cn.md) for the test taxonomy.

## ⑤ Register in support_list.md

```sh
python3 scripts/update_support_list.py docs/support_list.md
python3 scripts/check_modem_registry.py --fail-on-fixtures
```

`check_modem_registry.py` cross-checks all four registration sources
(modem_support.json, support_list.md, vendor function map, recognition
fixtures) and reports each gap individually.

## ⑥ PR checklist

- [ ] `modem_support.json` entry complete; `modes` matches the device (jq passes)
- [ ] `cmds/<vendor>.sh` contains only `cmd_*` wrappers; `sh -n` passes; no bashisms
- [ ] `vendor/<vendor>.sh` implements `get_imei`/`sim_info`/`base_info`/`network_info`, parsing via `ql_*`
- [ ] Unsupported features declared in `vendor_get_disabled_features`
- [ ] recognition fixtures committed with correct `expected_identity` (replay passes)
- [ ] `scripts/check_modem_registry.py` exits 0
- [ ] `docs/support_list.md` updated (script-generated)
- [ ] CI (ci.yml) green; hardware validation level stated

Run `git diff --check` before submitting to catch CRLF/trailing whitespace
(this repo is LF-only).
