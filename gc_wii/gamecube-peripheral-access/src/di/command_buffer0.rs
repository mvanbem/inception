#[doc = "Register `command_buffer0` reader"]
pub struct R(crate::R<COMMAND_BUFFER0_SPEC>);
impl core::ops::Deref for R {
    type Target = crate::R<COMMAND_BUFFER0_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<crate::R<COMMAND_BUFFER0_SPEC>> for R {
    #[inline(always)]
    fn from(reader: crate::R<COMMAND_BUFFER0_SPEC>) -> Self {
        R(reader)
    }
}
#[doc = "Register `command_buffer0` writer"]
pub struct W(crate::W<COMMAND_BUFFER0_SPEC>);
impl core::ops::Deref for W {
    type Target = crate::W<COMMAND_BUFFER0_SPEC>;
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
impl From<crate::W<COMMAND_BUFFER0_SPEC>> for W {
    #[inline(always)]
    fn from(writer: crate::W<COMMAND_BUFFER0_SPEC>) -> Self {
        W(writer)
    }
}
#[doc = "Field `command` reader - "]
pub struct COMMAND_R(crate::FieldReader<u8, u8>);
impl COMMAND_R {
    #[inline(always)]
    pub(crate) fn new(bits: u8) -> Self {
        COMMAND_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for COMMAND_R {
    type Target = crate::FieldReader<u8, u8>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `command` writer - "]
pub struct COMMAND_W<'a> {
    w: &'a mut W,
}
impl<'a> COMMAND_W<'a> {
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0xff << 24)) | ((value as u32 & 0xff) << 24);
        self.w
    }
}
#[doc = "Field `subcommand1` reader - "]
pub struct SUBCOMMAND1_R(crate::FieldReader<u8, u8>);
impl SUBCOMMAND1_R {
    #[inline(always)]
    pub(crate) fn new(bits: u8) -> Self {
        SUBCOMMAND1_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for SUBCOMMAND1_R {
    type Target = crate::FieldReader<u8, u8>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `subcommand1` writer - "]
pub struct SUBCOMMAND1_W<'a> {
    w: &'a mut W,
}
impl<'a> SUBCOMMAND1_W<'a> {
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0xff << 16)) | ((value as u32 & 0xff) << 16);
        self.w
    }
}
#[doc = "Field `subcommand2` reader - "]
pub struct SUBCOMMAND2_R(crate::FieldReader<u16, u16>);
impl SUBCOMMAND2_R {
    #[inline(always)]
    pub(crate) fn new(bits: u16) -> Self {
        SUBCOMMAND2_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for SUBCOMMAND2_R {
    type Target = crate::FieldReader<u16, u16>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `subcommand2` writer - "]
pub struct SUBCOMMAND2_W<'a> {
    w: &'a mut W,
}
impl<'a> SUBCOMMAND2_W<'a> {
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub unsafe fn bits(self, value: u16) -> &'a mut W {
        self.w.bits = (self.w.bits & !0xffff) | (value as u32 & 0xffff);
        self.w
    }
}
impl R {
    #[doc = "Bits 24:31"]
    #[inline(always)]
    pub fn command(&self) -> COMMAND_R {
        COMMAND_R::new(((self.bits >> 24) & 0xff) as u8)
    }
    #[doc = "Bits 16:23"]
    #[inline(always)]
    pub fn subcommand1(&self) -> SUBCOMMAND1_R {
        SUBCOMMAND1_R::new(((self.bits >> 16) & 0xff) as u8)
    }
    #[doc = "Bits 0:15"]
    #[inline(always)]
    pub fn subcommand2(&self) -> SUBCOMMAND2_R {
        SUBCOMMAND2_R::new((self.bits & 0xffff) as u16)
    }
}
impl W {
    #[doc = "Bits 24:31"]
    #[inline(always)]
    pub fn command(&mut self) -> COMMAND_W {
        COMMAND_W { w: self }
    }
    #[doc = "Bits 16:23"]
    #[inline(always)]
    pub fn subcommand1(&mut self) -> SUBCOMMAND1_W {
        SUBCOMMAND1_W { w: self }
    }
    #[doc = "Bits 0:15"]
    #[inline(always)]
    pub fn subcommand2(&mut self) -> SUBCOMMAND2_W {
        SUBCOMMAND2_W { w: self }
    }
    #[doc = "Writes raw bits to the register."]
    #[inline(always)]
    pub unsafe fn bits(&mut self, bits: u32) -> &mut Self {
        self.0.bits(bits);
        self
    }
}
#[doc = "\n\nThis register you can [`read`](crate::generic::Reg::read), [`write_with_zero`](crate::generic::Reg::write_with_zero), [`modify`](crate::generic::Reg::modify). See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [command_buffer0](index.html) module"]
pub struct COMMAND_BUFFER0_SPEC;
impl crate::RegisterSpec for COMMAND_BUFFER0_SPEC {
    type Ux = u32;
}
#[doc = "`read()` method returns [command_buffer0::R](R) reader structure"]
impl crate::Readable for COMMAND_BUFFER0_SPEC {
    type Reader = R;
}
#[doc = "`write(|w| ..)` method takes [command_buffer0::W](W) writer structure"]
impl crate::Writable for COMMAND_BUFFER0_SPEC {
    type Writer = W;
}
