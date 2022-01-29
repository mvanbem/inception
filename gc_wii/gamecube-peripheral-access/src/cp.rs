#[doc = r"Register block"]
#[repr(C)]
pub struct RegisterBlock {
    _reserved0: [u8; 0x40],
    #[doc = "0x40 - "]
    pub xf_rasbusy_l: crate::Reg<xf_rasbusy_l::XF_RASBUSY_L_SPEC>,
    #[doc = "0x42 - "]
    pub xf_rasbusy_h: crate::Reg<xf_rasbusy_h::XF_RASBUSY_H_SPEC>,
    #[doc = "0x44 - "]
    pub xf_clks_l: crate::Reg<xf_clks_l::XF_CLKS_L_SPEC>,
    #[doc = "0x46 - "]
    pub xf_clks_h: crate::Reg<xf_clks_h::XF_CLKS_H_SPEC>,
    #[doc = "0x48 - "]
    pub xf_wait_in_l: crate::Reg<xf_wait_in_l::XF_WAIT_IN_L_SPEC>,
    #[doc = "0x4a - "]
    pub xf_wait_in_h: crate::Reg<xf_wait_in_h::XF_WAIT_IN_H_SPEC>,
    #[doc = "0x4c - "]
    pub xf_wait_out_l: crate::Reg<xf_wait_out_l::XF_WAIT_OUT_L_SPEC>,
    #[doc = "0x4e - "]
    pub xf_wait_out_h: crate::Reg<xf_wait_out_h::XF_WAIT_OUT_H_SPEC>,
    #[doc = "0x50 - "]
    pub vcache_metric_check_l: crate::Reg<vcache_metric_check_l::VCACHE_METRIC_CHECK_L_SPEC>,
    #[doc = "0x52 - "]
    pub vcache_metric_check_h: crate::Reg<vcache_metric_check_h::VCACHE_METRIC_CHECK_H_SPEC>,
    #[doc = "0x54 - "]
    pub vcache_metric_miss_l: crate::Reg<vcache_metric_miss_l::VCACHE_METRIC_MISS_L_SPEC>,
    #[doc = "0x56 - "]
    pub vcache_metric_miss_h: crate::Reg<vcache_metric_miss_h::VCACHE_METRIC_MISS_H_SPEC>,
    #[doc = "0x58 - "]
    pub vcache_metric_stall_l: crate::Reg<vcache_metric_stall_l::VCACHE_METRIC_STALL_L_SPEC>,
    #[doc = "0x5a - "]
    pub vcache_metric_stall_h: crate::Reg<vcache_metric_stall_h::VCACHE_METRIC_STALL_H_SPEC>,
    _reserved14: [u8; 0x04],
    #[doc = "0x60 - "]
    pub clks_per_vtx_in_l: crate::Reg<clks_per_vtx_in_l::CLKS_PER_VTX_IN_L_SPEC>,
    #[doc = "0x62 - "]
    pub clks_per_vtx_in_h: crate::Reg<clks_per_vtx_in_h::CLKS_PER_VTX_IN_H_SPEC>,
    #[doc = "0x64 - "]
    pub clks_per_vtx_out: crate::Reg<clks_per_vtx_out::CLKS_PER_VTX_OUT_SPEC>,
}
#[doc = "XF_RASBUSY_L register accessor: an alias for `Reg<XF_RASBUSY_L_SPEC>`"]
pub type XF_RASBUSY_L = crate::Reg<xf_rasbusy_l::XF_RASBUSY_L_SPEC>;
#[doc = ""]
pub mod xf_rasbusy_l;
#[doc = "XF_RASBUSY_H register accessor: an alias for `Reg<XF_RASBUSY_H_SPEC>`"]
pub type XF_RASBUSY_H = crate::Reg<xf_rasbusy_h::XF_RASBUSY_H_SPEC>;
#[doc = ""]
pub mod xf_rasbusy_h;
#[doc = "XF_CLKS_L register accessor: an alias for `Reg<XF_CLKS_L_SPEC>`"]
pub type XF_CLKS_L = crate::Reg<xf_clks_l::XF_CLKS_L_SPEC>;
#[doc = ""]
pub mod xf_clks_l;
#[doc = "XF_CLKS_H register accessor: an alias for `Reg<XF_CLKS_H_SPEC>`"]
pub type XF_CLKS_H = crate::Reg<xf_clks_h::XF_CLKS_H_SPEC>;
#[doc = ""]
pub mod xf_clks_h;
#[doc = "XF_WAIT_IN_L register accessor: an alias for `Reg<XF_WAIT_IN_L_SPEC>`"]
pub type XF_WAIT_IN_L = crate::Reg<xf_wait_in_l::XF_WAIT_IN_L_SPEC>;
#[doc = ""]
pub mod xf_wait_in_l;
#[doc = "XF_WAIT_IN_H register accessor: an alias for `Reg<XF_WAIT_IN_H_SPEC>`"]
pub type XF_WAIT_IN_H = crate::Reg<xf_wait_in_h::XF_WAIT_IN_H_SPEC>;
#[doc = ""]
pub mod xf_wait_in_h;
#[doc = "XF_WAIT_OUT_L register accessor: an alias for `Reg<XF_WAIT_OUT_L_SPEC>`"]
pub type XF_WAIT_OUT_L = crate::Reg<xf_wait_out_l::XF_WAIT_OUT_L_SPEC>;
#[doc = ""]
pub mod xf_wait_out_l;
#[doc = "XF_WAIT_OUT_H register accessor: an alias for `Reg<XF_WAIT_OUT_H_SPEC>`"]
pub type XF_WAIT_OUT_H = crate::Reg<xf_wait_out_h::XF_WAIT_OUT_H_SPEC>;
#[doc = ""]
pub mod xf_wait_out_h;
#[doc = "VCACHE_METRIC_CHECK_L register accessor: an alias for `Reg<VCACHE_METRIC_CHECK_L_SPEC>`"]
pub type VCACHE_METRIC_CHECK_L = crate::Reg<vcache_metric_check_l::VCACHE_METRIC_CHECK_L_SPEC>;
#[doc = ""]
pub mod vcache_metric_check_l;
#[doc = "VCACHE_METRIC_CHECK_H register accessor: an alias for `Reg<VCACHE_METRIC_CHECK_H_SPEC>`"]
pub type VCACHE_METRIC_CHECK_H = crate::Reg<vcache_metric_check_h::VCACHE_METRIC_CHECK_H_SPEC>;
#[doc = ""]
pub mod vcache_metric_check_h;
#[doc = "VCACHE_METRIC_MISS_L register accessor: an alias for `Reg<VCACHE_METRIC_MISS_L_SPEC>`"]
pub type VCACHE_METRIC_MISS_L = crate::Reg<vcache_metric_miss_l::VCACHE_METRIC_MISS_L_SPEC>;
#[doc = ""]
pub mod vcache_metric_miss_l;
#[doc = "VCACHE_METRIC_MISS_H register accessor: an alias for `Reg<VCACHE_METRIC_MISS_H_SPEC>`"]
pub type VCACHE_METRIC_MISS_H = crate::Reg<vcache_metric_miss_h::VCACHE_METRIC_MISS_H_SPEC>;
#[doc = ""]
pub mod vcache_metric_miss_h;
#[doc = "VCACHE_METRIC_STALL_L register accessor: an alias for `Reg<VCACHE_METRIC_STALL_L_SPEC>`"]
pub type VCACHE_METRIC_STALL_L = crate::Reg<vcache_metric_stall_l::VCACHE_METRIC_STALL_L_SPEC>;
#[doc = ""]
pub mod vcache_metric_stall_l;
#[doc = "VCACHE_METRIC_STALL_H register accessor: an alias for `Reg<VCACHE_METRIC_STALL_H_SPEC>`"]
pub type VCACHE_METRIC_STALL_H = crate::Reg<vcache_metric_stall_h::VCACHE_METRIC_STALL_H_SPEC>;
#[doc = ""]
pub mod vcache_metric_stall_h;
#[doc = "CLKS_PER_VTX_IN_L register accessor: an alias for `Reg<CLKS_PER_VTX_IN_L_SPEC>`"]
pub type CLKS_PER_VTX_IN_L = crate::Reg<clks_per_vtx_in_l::CLKS_PER_VTX_IN_L_SPEC>;
#[doc = ""]
pub mod clks_per_vtx_in_l;
#[doc = "CLKS_PER_VTX_IN_H register accessor: an alias for `Reg<CLKS_PER_VTX_IN_H_SPEC>`"]
pub type CLKS_PER_VTX_IN_H = crate::Reg<clks_per_vtx_in_h::CLKS_PER_VTX_IN_H_SPEC>;
#[doc = ""]
pub mod clks_per_vtx_in_h;
#[doc = "CLKS_PER_VTX_OUT register accessor: an alias for `Reg<CLKS_PER_VTX_OUT_SPEC>`"]
pub type CLKS_PER_VTX_OUT = crate::Reg<clks_per_vtx_out::CLKS_PER_VTX_OUT_SPEC>;
#[doc = ""]
pub mod clks_per_vtx_out;
