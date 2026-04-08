declare namespace Openness.open {

    interface LoginConfigRegister {
        emailSuffix  :string   // 注册邮箱后缀
        openRegister :boolean    // 开放注册
    }

    interface LoginConfigPasswordPolicy {
        allowWeakPassword: boolean
    }
    
    interface LoginVcodeResponse{
        loginCaptcha: boolean
        register:LoginConfigRegister
        passwordPolicy?: LoginConfigPasswordPolicy
    }
}
