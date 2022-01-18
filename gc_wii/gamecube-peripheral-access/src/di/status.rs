#[doc = "Register `status` reader"]
pub struct R(crate::R<STATUS_SPEC>);
impl core::ops::Deref for R {
    type Target = crate::R<STATUS_SPEC>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl From<crate::R<STATUS_SPEC>> for R {
    #[inline(always)]
    fn from(reader: crate::R<STATUS_SPEC>) -> Self {
        R(reader)
    }
}
#[doc = "Register `status` writer"]
pub struct W(crate::W<STATUS_SPEC>);
impl core::ops::Deref for W {
    type Target = crate::W<STATUS_SPEC>;
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
impl From<crate::W<STATUS_SPEC>> for W {
    #[inline(always)]
    fn from(writer: crate::W<STATUS_SPEC>) -> Self {
        W(writer)
    }
}
#[doc = "Field `break_complete_asserted` reader - "]
pub struct BREAK_COMPLETE_ASSERTED_R(crate::FieldReader<bool, bool>);
impl BREAK_COMPLETE_ASSERTED_R {
    #[inline(always)]
    pub(crate) fn new(bits: bool) -> Self {
        BREAK_COMPLETE_ASSERTED_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for BREAK_COMPLETE_ASSERTED_R {
    type Target = crate::FieldReader<bool, bool>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `ack_break_complete` writer - "]
pub struct ACK_BREAK_COMPLETE_W<'a> {
    w: &'a mut W,
}
impl<'a> ACK_BREAK_COMPLETE_W<'a> {
    #[doc = r"Sets the field bit"]
    #[inline(always)]
    pub fn set_bit(self) -> &'a mut W {
        self.bit(true)
    }
    #[doc = r"Clears the field bit"]
    #[inline(always)]
    pub fn clear_bit(self) -> &'a mut W {
        self.bit(false)
    }
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub fn bit(self, value: bool) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0x01 << 6)) | ((value as u32 & 0x01) << 6);
        self.w
    }
}
#[doc = "Field `break_complete_mask` reader - "]
pub struct BREAK_COMPLETE_MASK_R(crate::FieldReader<bool, bool>);
impl BREAK_COMPLETE_MASK_R {
    #[inline(always)]
    pub(crate) fn new(bits: bool) -> Self {
        BREAK_COMPLETE_MASK_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for BREAK_COMPLETE_MASK_R {
    type Target = crate::FieldReader<bool, bool>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `break_complete_mask` writer - "]
pub struct BREAK_COMPLETE_MASK_W<'a> {
    w: &'a mut W,
}
impl<'a> BREAK_COMPLETE_MASK_W<'a> {
    #[doc = r"Sets the field bit"]
    #[inline(always)]
    pub fn set_bit(self) -> &'a mut W {
        self.bit(true)
    }
    #[doc = r"Clears the field bit"]
    #[inline(always)]
    pub fn clear_bit(self) -> &'a mut W {
        self.bit(false)
    }
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub fn bit(self, value: bool) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0x01 << 5)) | ((value as u32 & 0x01) << 5);
        self.w
    }
}
#[doc = "Field `transfer_complete_asserted` reader - "]
pub struct TRANSFER_COMPLETE_ASSERTED_R(crate::FieldReader<bool, bool>);
impl TRANSFER_COMPLETE_ASSERTED_R {
    #[inline(always)]
    pub(crate) fn new(bits: bool) -> Self {
        TRANSFER_COMPLETE_ASSERTED_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for TRANSFER_COMPLETE_ASSERTED_R {
    type Target = crate::FieldReader<bool, bool>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `ack_transfer_complete` writer - "]
pub struct ACK_TRANSFER_COMPLETE_W<'a> {
    w: &'a mut W,
}
impl<'a> ACK_TRANSFER_COMPLETE_W<'a> {
    #[doc = r"Sets the field bit"]
    #[inline(always)]
    pub fn set_bit(self) -> &'a mut W {
        self.bit(true)
    }
    #[doc = r"Clears the field bit"]
    #[inline(always)]
    pub fn clear_bit(self) -> &'a mut W {
        self.bit(false)
    }
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub fn bit(self, value: bool) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0x01 << 4)) | ((value as u32 & 0x01) << 4);
        self.w
    }
}
#[doc = "Field `transfer_complete_mask` reader - "]
pub struct TRANSFER_COMPLETE_MASK_R(crate::FieldReader<bool, bool>);
impl TRANSFER_COMPLETE_MASK_R {
    #[inline(always)]
    pub(crate) fn new(bits: bool) -> Self {
        TRANSFER_COMPLETE_MASK_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for TRANSFER_COMPLETE_MASK_R {
    type Target = crate::FieldReader<bool, bool>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `transfer_complete_mask` writer - "]
pub struct TRANSFER_COMPLETE_MASK_W<'a> {
    w: &'a mut W,
}
impl<'a> TRANSFER_COMPLETE_MASK_W<'a> {
    #[doc = r"Sets the field bit"]
    #[inline(always)]
    pub fn set_bit(self) -> &'a mut W {
        self.bit(true)
    }
    #[doc = r"Clears the field bit"]
    #[inline(always)]
    pub fn clear_bit(self) -> &'a mut W {
        self.bit(false)
    }
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub fn bit(self, value: bool) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0x01 << 3)) | ((value as u32 & 0x01) << 3);
        self.w
    }
}
#[doc = "Field `device_error_asserted` reader - "]
pub struct DEVICE_ERROR_ASSERTED_R(crate::FieldReader<bool, bool>);
impl DEVICE_ERROR_ASSERTED_R {
    #[inline(always)]
    pub(crate) fn new(bits: bool) -> Self {
        DEVICE_ERROR_ASSERTED_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for DEVICE_ERROR_ASSERTED_R {
    type Target = crate::FieldReader<bool, bool>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `ack_device_error` writer - "]
pub struct ACK_DEVICE_ERROR_W<'a> {
    w: &'a mut W,
}
impl<'a> ACK_DEVICE_ERROR_W<'a> {
    #[doc = r"Sets the field bit"]
    #[inline(always)]
    pub fn set_bit(self) -> &'a mut W {
        self.bit(true)
    }
    #[doc = r"Clears the field bit"]
    #[inline(always)]
    pub fn clear_bit(self) -> &'a mut W {
        self.bit(false)
    }
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub fn bit(self, value: bool) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0x01 << 2)) | ((value as u32 & 0x01) << 2);
        self.w
    }
}
#[doc = "Field `device_error_mask` reader - "]
pub struct DEVICE_ERROR_MASK_R(crate::FieldReader<bool, bool>);
impl DEVICE_ERROR_MASK_R {
    #[inline(always)]
    pub(crate) fn new(bits: bool) -> Self {
        DEVICE_ERROR_MASK_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for DEVICE_ERROR_MASK_R {
    type Target = crate::FieldReader<bool, bool>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `device_error_mask` writer - "]
pub struct DEVICE_ERROR_MASK_W<'a> {
    w: &'a mut W,
}
impl<'a> DEVICE_ERROR_MASK_W<'a> {
    #[doc = r"Sets the field bit"]
    #[inline(always)]
    pub fn set_bit(self) -> &'a mut W {
        self.bit(true)
    }
    #[doc = r"Clears the field bit"]
    #[inline(always)]
    pub fn clear_bit(self) -> &'a mut W {
        self.bit(false)
    }
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub fn bit(self, value: bool) -> &'a mut W {
        self.w.bits = (self.w.bits & !(0x01 << 1)) | ((value as u32 & 0x01) << 1);
        self.w
    }
}
#[doc = "Field `break_pending` reader - "]
pub struct BREAK_PENDING_R(crate::FieldReader<bool, bool>);
impl BREAK_PENDING_R {
    #[inline(always)]
    pub(crate) fn new(bits: bool) -> Self {
        BREAK_PENDING_R(crate::FieldReader::new(bits))
    }
}
impl core::ops::Deref for BREAK_PENDING_R {
    type Target = crate::FieldReader<bool, bool>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[doc = "Field `request_break` writer - "]
pub struct REQUEST_BREAK_W<'a> {
    w: &'a mut W,
}
impl<'a> REQUEST_BREAK_W<'a> {
    #[doc = r"Sets the field bit"]
    #[inline(always)]
    pub fn set_bit(self) -> &'a mut W {
        self.bit(true)
    }
    #[doc = r"Clears the field bit"]
    #[inline(always)]
    pub fn clear_bit(self) -> &'a mut W {
        self.bit(false)
    }
    #[doc = r"Writes raw bits to the field"]
    #[inline(always)]
    pub fn bit(self, value: bool) -> &'a mut W {
        self.w.bits = (self.w.bits & !0x01) | (value as u32 & 0x01);
        self.w
    }
}
impl R {
    #[doc = "Bit 6"]
    #[inline(always)]
    pub fn break_complete_asserted(&self) -> BREAK_COMPLETE_ASSERTED_R {
        BREAK_COMPLETE_ASSERTED_R::new(((self.bits >> 6) & 0x01) != 0)
    }
    #[doc = "Bit 5"]
    #[inline(always)]
    pub fn break_complete_mask(&self) -> BREAK_COMPLETE_MASK_R {
        BREAK_COMPLETE_MASK_R::new(((self.bits >> 5) & 0x01) != 0)
    }
    #[doc = "Bit 4"]
    #[inline(always)]
    pub fn transfer_complete_asserted(&self) -> TRANSFER_COMPLETE_ASSERTED_R {
        TRANSFER_COMPLETE_ASSERTED_R::new(((self.bits >> 4) & 0x01) != 0)
    }
    #[doc = "Bit 3"]
    #[inline(always)]
    pub fn transfer_complete_mask(&self) -> TRANSFER_COMPLETE_MASK_R {
        TRANSFER_COMPLETE_MASK_R::new(((self.bits >> 3) & 0x01) != 0)
    }
    #[doc = "Bit 2"]
    #[inline(always)]
    pub fn device_error_asserted(&self) -> DEVICE_ERROR_ASSERTED_R {
        DEVICE_ERROR_ASSERTED_R::new(((self.bits >> 2) & 0x01) != 0)
    }
    #[doc = "Bit 1"]
    #[inline(always)]
    pub fn device_error_mask(&self) -> DEVICE_ERROR_MASK_R {
        DEVICE_ERROR_MASK_R::new(((self.bits >> 1) & 0x01) != 0)
    }
    #[doc = "Bit 0"]
    #[inline(always)]
    pub fn break_pending(&self) -> BREAK_PENDING_R {
        BREAK_PENDING_R::new((self.bits & 0x01) != 0)
    }
}
impl W {
    #[doc = "Bit 6"]
    #[inline(always)]
    pub fn ack_break_complete(&mut self) -> ACK_BREAK_COMPLETE_W {
        ACK_BREAK_COMPLETE_W { w: self }
    }
    #[doc = "Bit 5"]
    #[inline(always)]
    pub fn break_complete_mask(&mut self) -> BREAK_COMPLETE_MASK_W {
        BREAK_COMPLETE_MASK_W { w: self }
    }
    #[doc = "Bit 4"]
    #[inline(always)]
    pub fn ack_transfer_complete(&mut self) -> ACK_TRANSFER_COMPLETE_W {
        ACK_TRANSFER_COMPLETE_W { w: self }
    }
    #[doc = "Bit 3"]
    #[inline(always)]
    pub fn transfer_complete_mask(&mut self) -> TRANSFER_COMPLETE_MASK_W {
        TRANSFER_COMPLETE_MASK_W { w: self }
    }
    #[doc = "Bit 2"]
    #[inline(always)]
    pub fn ack_device_error(&mut self) -> ACK_DEVICE_ERROR_W {
        ACK_DEVICE_ERROR_W { w: self }
    }
    #[doc = "Bit 1"]
    #[inline(always)]
    pub fn device_error_mask(&mut self) -> DEVICE_ERROR_MASK_W {
        DEVICE_ERROR_MASK_W { w: self }
    }
    #[doc = "Bit 0"]
    #[inline(always)]
    pub fn request_break(&mut self) -> REQUEST_BREAK_W {
        REQUEST_BREAK_W { w: self }
    }
    #[doc = "Writes raw bits to the register."]
    #[inline(always)]
    pub unsafe fn bits(&mut self, bits: u32) -> &mut Self {
        self.0.bits(bits);
        self
    }
}
#[doc = "\n\nThis register you can [`read`](crate::generic::Reg::read), [`write_with_zero`](crate::generic::Reg::write_with_zero), [`modify`](crate::generic::Reg::modify). See [API](https://docs.rs/svd2rust/#read--modify--write-api).\n\nFor information about available fields see [status](index.html) module"]
pub struct STATUS_SPEC;
impl crate::RegisterSpec for STATUS_SPEC {
    type Ux = u32;
}
#[doc = "`read()` method returns [status::R](R) reader structure"]
impl crate::Readable for STATUS_SPEC {
    type Reader = R;
}
#[doc = "`write(|w| ..)` method takes [status::W](W) writer structure"]
impl crate::Writable for STATUS_SPEC {
    type Writer = W;
}
