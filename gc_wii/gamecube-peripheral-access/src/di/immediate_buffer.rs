#[doc = "Register `immediate_buffer` reader"]
pub struct R(crate::R<IMMEDIATE_BUFFER_SPEC>);
impl core::ops::Deref for R {
    type Target = crate::R<IMMEDIATE_BUFFER_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<crate::R<IMMEDIATE_BUFFER_SPEC>> for R {
    #[inline(always)]
    fn from(reader: crate::R<IMMEDIATE_BUFFER_SPEC>) -> Self {
        R(reader)
    }
}
#[doc = "Register `immediate_buffer` writer"]
pub struct W(crate::W<IMMEDIATE_BUFFER_SPEC>);
impl core::ops::Deref for W {
    type Target = crate::W<IMMEDIATE_BUFFER_SPEC>;
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
impl From<crate::W<IMMEDIATE_BUFFER_SPEC>> for W {
    #[inline(always)]
    fn from(writer: crate::W<IMMEDIATE_BUFFER_SPEC>) -> Self {
        W(writer)
    }
}
#[doc = "Field `reg_val0` reader - "]
pub struct REG_VAL0_R(crate::FieldReader<u8, u8>);
impl REG_VAL0_R {
    #[inline(always)]
    pub(crate) fn new(bits: u8) -> Self {
        REG_VAL0_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for REG_VAL0_R {
    type Target = crate::FieldReader<u8, u8>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `reg_val0` writer - "]
pub struct REG_VAL0_W<'a> {
    w: &'a mut W,
}
impl<'a> REG_VAL0_W<'a> {
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0xff << 24)) | ((value as u32 & 0xff) << 24);
        self.w
    }
}
#[doc = "Field `reg_val1` reader - "]
pub struct REG_VAL1_R(crate::FieldReader<u8, u8>);
impl REG_VAL1_R {
    #[inline(always)]
    pub(crate) fn new(bits: u8) -> Self {
        REG_VAL1_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for REG_VAL1_R {
    type Target = crate::FieldReader<u8, u8>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `reg_val1` writer - "]
pub struct REG_VAL1_W<'a> {
    w: &'a mut W,
}
impl<'a> REG_VAL1_W<'a> {
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0xff << 16)) | ((value as u32 & 0xff) << 16);
        self.w
    }
}
#[doc = "Field `reg_val2` reader - "]
pub struct REG_VAL2_R(crate::FieldReader<u8, u8>);
impl REG_VAL2_R {
    #[inline(always)]
    pub(crate) fn new(bits: u8) -> Self {
        REG_VAL2_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for REG_VAL2_R {
    type Target = crate::FieldReader<u8, u8>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `reg_val2` writer - "]
pub struct REG_VAL2_W<'a> {
    w: &'a mut W,
}
impl<'a> REG_VAL2_W<'a> {
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0xff << 8)) | ((value as u32 & 0xff) << 8);
        self.w
    }
}
#[doc = "Field `reg_val3` reader - "]
pub struct REG_VAL3_R(crate::FieldReader<u8, u8>);
impl REG_VAL3_R {
    #[inline(always)]
    pub(crate) fn new(bits: u8) -> Self {
        REG_VAL3_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for REG_VAL3_R {
    type Target = crate::FieldReader<u8, u8>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `reg_val3` writer - "]
pub struct REG_VAL3_W<'a> {
    w: &'a mut W,
}
impl<'a> REG_VAL3_W<'a> {
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut W {
        self.w.bits = (self.w.bits & !0xff) | (value as u32 & 0xff);
        self.w
    }
}
impl R {
    #[doc = "Bits 24:31"]
    #[inline(always)]
    pub fn reg_val0(&self) -> REG_VAL0_R {
        REG_VAL0_R::new(((self.bits >> 24) & 0xff) as u8)
    }
    #[doc = "Bits 16:23"]
    #[inline(always)]
    pub fn reg_val1(&self) -> REG_VAL1_R {
        REG_VAL1_R::new(((self.bits >> 16) & 0xff) as u8)
    }
    #[doc = "Bits 8:15"]
    #[inline(always)]
    pub fn reg_val2(&self) -> REG_VAL2_R {
        REG_VAL2_R::new(((self.bits >> 8) & 0xff) as u8)
    }
    #[doc = "Bits 0:7"]
    #[inline(always)]
    pub fn reg_val3(&self) -> REG_VAL3_R {
        REG_VAL3_R::new((self.bits & 0xff) as u8)
    }
}
impl W {
    #[doc = "Bits 24:31"]
    #[inline(always)]
    pub fn reg_val0(&mut self) -> REG_VAL0_W {
        REG_VAL0_W { w: self }
    }
    #[doc = "Bits 16:23"]
    #[inline(always)]
    pub fn reg_val1(&mut self) -> REG_VAL1_W {
        REG_VAL1_W { w: self }
    }
    #[doc = "Bits 8:15"]
    #[inline(always)]
    pub fn reg_val2(&mut self) -> REG_VAL2_W {
        REG_VAL2_W { w: self }
    }
    #[doc = "Bits 0:7"]
    #[inline(always)]
    pub fn reg_val3(&mut self) -> REG_VAL3_W {
        REG_VAL3_W { w: self }
    }
    #[doc = "Writes raw bits to the register."]
    #[inline(always)]
    pub unsafe fn bits(&mut self, bits: u32) -> &mut Self {
        self.0.bits(bits);
        self
    }
}
#[doc = "\n\nThis register you can [`read`](crate::generic::Reg::read), [`write_with_zero`](crate::generic::Reg::write_with_zero), [`modify`](crate::generic::Reg::modify). See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [immediate_buffer](index.html) module"]
pub struct IMMEDIATE_BUFFER_SPEC;
impl crate::RegisterSpec for IMMEDIATE_BUFFER_SPEC {
    type Ux = u32;
}
#[doc = "`read()` method returns [immediate_buffer::R](R) reader structure"]
impl crate::Readable for IMMEDIATE_BUFFER_SPEC {
    type Reader = R;
}
#[doc = "`write(|w| ..)` method takes [immediate_buffer::W](W) writer structure"]
impl crate::Writable for IMMEDIATE_BUFFER_SPEC {
    type Writer = W;
}
