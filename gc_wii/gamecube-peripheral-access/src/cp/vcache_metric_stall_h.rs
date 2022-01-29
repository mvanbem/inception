#[doc = "Register `VCACHE_METRIC_STALL_H` reader"]
pub struct R(crate::R<VCACHE_METRIC_STALL_H_SPEC>);
impl core::ops::Deref for R {
    type Target = crate::R<VCACHE_METRIC_STALL_H_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<crate::R<VCACHE_METRIC_STALL_H_SPEC>> for R {
    #[inline(always)]
    fn from(reader: crate::R<VCACHE_METRIC_STALL_H_SPEC>) -> Self {
        R(reader)
    }
}
#[doc = "Register `VCACHE_METRIC_STALL_H` writer"]
pub struct W(crate::W<VCACHE_METRIC_STALL_H_SPEC>);
impl core::ops::Deref for W {
    type Target = crate::W<VCACHE_METRIC_STALL_H_SPEC>;
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
impl From<crate::W<VCACHE_METRIC_STALL_H_SPEC>> for W {
    #[inline(always)]
    fn from(writer: crate::W<VCACHE_METRIC_STALL_H_SPEC>) -> Self {
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
#[doc = "\n\nThis register you can [`read`](crate::generic::Reg::read), [`write_with_zero`](crate::generic::Reg::write_with_zero), [`modify`](crate::generic::Reg::modify). See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [vcache_metric_stall_h](index.html) module"]
pub struct VCACHE_METRIC_STALL_H_SPEC;
impl crate::RegisterSpec for VCACHE_METRIC_STALL_H_SPEC {
    type Ux = u16;
}
#[doc = "`read()` method returns [vcache_metric_stall_h::R](R) reader structure"]
impl crate::Readable for VCACHE_METRIC_STALL_H_SPEC {
    type Reader = R;
}
#[doc = "`write(|w| ..)` method takes [vcache_metric_stall_h::W](W) writer structure"]
impl crate::Writable for VCACHE_METRIC_STALL_H_SPEC {
    type Writer = W;
}
