# 模组注册一致性基线（首跑）

> 生成方式：`python3 scripts/check_modem_registry.py --report --baseline`（agent-support-sync，2026-10-12）
>
> 结论：`modem_support.json` 与 `docs/support_list.md` 完全对齐（91/91）；schema 校验 0 错误。
> 存量不一致仅 2 处 vendor 函数缺口（见下）。recognition fixture 覆盖为增量建设目标（2/91），
> 默认不计入门禁 FAIL（`--fail-on-fixtures` 可开启）。
>
> 转严格模式步骤：
> 1. 修复 `vendor/neoway.sh` 缺失的 `get_mode()` 与 `vendor/telit.sh` 缺失的 `get_imei()`
>    （或经评审后从 `scripts/check_modem_registry.py` 的 `REQUIRED_VENDOR_FUNCS` 调整必需集）；
> 2. 将 `.github/workflows/quality.yml` 中 `modem-registry` stage 的 `--baseline` 参数移除；
> 3. 此后任何一处登记遗漏（json/md/vendor 不一致）即门禁 FAIL。
> 说明：`--baseline` 模式仅报告不 FAIL；严格模式（无参数）退出码 0/1。

# Modem registry consistency report

- json entries (usb/pcie): 91
- support_list.md rows: 91
- schema errors: 0
- vendor problems: 2
- fixture missing: 89

| Category | Count |
| --- | --- |
| in json not in md | 0 |
| in md not in json | 0 |
| vendor function/mapping missing | 2 |
| recognition fixture missing | 89 |

## In json but not in md (0)

- (none)

## In md but not in json (0)

- (none)

## Vendor function/mapping problems (2)

- vendor/neoway.sh: missing required functions: get_mode
- vendor/telit.sh: missing required functions: get_imei

## Schema errors (0)

- (none)

## Recognition fixture missing (89)

- fibocom/qualcomm/fm150-ae
- fibocom/qualcomm/fm160-cn
- fibocom/qualcomm/fm160-eau
- fibocom/qualcomm/fm160-na
- fibocom/qualcomm/fm170-eau
- fibocom/qualcomm/fm190w-gl
- fibocom/mediatek/fm350-gl
- fibocom/mediatek/fm350-gl-00
- fibocom/mediatek/fm350r-gl
- fibocom/unisoc/fm650-cn
- fibocom/unisoc/l610-eu
- fibocom/lte/nl668
- fibocom/lte/nl678
- fibocom/mediatek/rw350r-gl
- foxconn/qualcomm/t77w968
- foxconn/qualcomm/t99w175
- foxconn/qualcomm/t99w368
- foxconn/qualcomm/t99w373
- foxconn/qualcomm/t99w640
- gosuncn/qualcomm/gm800
- gosuncn/lte/me3630-w
- huawei/hisilicon/mh5000-31
- huawei/unisoc/mh5000-82
- huawei/unisoc/mh5000-82m
- huawei/hisilicon/mt5700m-cn
- huawei/hisilicon/mt5710_cn
- huawei/hisilicon/s10
- meig/qualcomm/srm815
- meig/unisoc/srm821
- meig/qualcomm/srm825
- meig/qualcomm/srm825n
- neoway/unisoc/n510m
- quectel/qualcomm/ag598eeu
- quectel/lte/ec200a
- quectel/lte/ec20f
- quectel/lte/ec21
- quectel/lte/ec25
- quectel/lte/eg25
- quectel/lte/em05
- quectel/lte/em05-cn
- quectel/lte/em12
- quectel/lte12/em120k
- quectel/lte12/em160r-gl
- quectel/lte/ep06
- quectel/qualcomm/mh8001-eu
- quectel/unisoc/rg200u-cn
- quectel/qualcomm/rg500q-ea
- quectel/qualcomm/rg501q-eu
- quectel/qualcomm/rg520f-eb
- quectel/qualcomm/rg520f-eu
- quectel/qualcomm/rg520f-jo
- quectel/qualcomm/rg520n-eb
- quectel/qualcomm/rg520n-eu
- quectel/qualcomm/rg530f-na
- quectel/qualcomm/rg650v-eu
- quectel/hisilicon/rg801
- quectel/qualcomm/rm500q-ae
- quectel/qualcomm/rm500q-cn
- quectel/qualcomm/rm500q-gl
- quectel/qualcomm/rm500s-ce
- quectel/unisoc/rm500u-cn
- quectel/unisoc/rm500u-ea
- quectel/qualcomm/rm502q-ae
- quectel/qualcomm/rm502q-gl
- quectel/qualcomm/rm505q-ae
- quectel/qualcomm/rm510q-gl
- quectel/qualcomm/rm510q-glha
- quectel/qualcomm/rm520f-bl
- quectel/qualcomm/rm520f-gl
- quectel/qualcomm/rm520n-cn
- quectel/qualcomm/rm520n-eu
- quectel/qualcomm/rm520n-gl
- quectel/qualcomm/rm521f-gl
- quectel/qualcomm/rm530n-gl
- quectel/qualcomm/rm551e-gl
- sierra/qualcomm/em9190
- sierra/qualcomm/em9291
- sierra/lte/mc7354
- sierra/lte/mc7355
- simcom/lte/a7605c
- simcom/asrmicro/a8200c-m2
- simcom/asrmicro/simcom_a8200_serias
- simcom/qualcomm/simcom_d8200g
- simcom/qualcomm/simcom_sim8200ea-m2
- simcom/qualcomm/simcom_sim8202g-m2
- simcom/qualcomm/simcom_sim8380g-m2
- telit/qualcomm/fn990a40
- thales/qualcomm/mv32-w-a
- thales/qualcomm/mv32-w-b

