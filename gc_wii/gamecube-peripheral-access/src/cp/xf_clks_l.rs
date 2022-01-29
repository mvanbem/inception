#[doc = "Register `XF_CLKS_L` reader"]
pub struct R(crate::R<XF_CLKS_L_SPEC>);
impl core::ops::Deref for R {
    type Target = crate::R<XF_CLKS_L_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<crate::R<XF_CLKS_L_SPEC>> for R {
    #[inline(always)]
    fn from(reader: crate::R<XF_CLKS_L_SPEC>) -> Self {
        R(reader)
    }
}
#[doc = "Register `XF_CLKS_L` writer"]
pub struct W(crate::W<XF_CLKS_L_SPEC>);
impl core::ops::Deref for W {
    type Target = crate::W<XF_CLKS_L_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl core::ops::DerefMut for W {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl From<crate::W<XF_CLKS_L_SPEC>> for W {
    #[inline(always)]
    fn from(writer: crate::W<XF_CLKS_L_SPEC>) -> Self {
        W(writer)
    }
}
impl W {
    #[doc = "Writes raw bits to the register."]
    #[inline(always)]
    pub unsafe fn bits(&mut self, bits: u16) -> &mut Self {
        self.0.bits(bits);
        self
    }
}
#[doc = "\n\nThis register you can [`read`](crate::generic::Reg::read), [`write_with_zero`](crate::generic::Reg::write_with_zero), [`modify`](crate::generic::Reg::modify). See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [xf_clks_l](index.html) module"]
pub struct XF_CLKS_L_SPEC;
impl crate::RegisterSpec for XF_CLKS_L_SPEC {
    type Ux = u16;
}
#[doc = "`read()` method returns [xf_clks_l::R](R) reader structure"]
impl crate::Readable for XF_CLKS_L_SPEC {
    type Reader = R;
}
#[doc = "`write(|w| ..)` method takes [xf_clks_l::W](W) writer structure"]
impl crate::Writable for XF_CLKS_L_SPEC {
    type Writer = W;
}
