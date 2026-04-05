declare namespace Login{

    interface LoginReqest{
        username:string 
        password:string
        vcode?:string
    }

	interface LoginResponse extends User.Info{
	}

    interface ResetPasswordByVCodeReqest extends System.Register.SendRegisterVcodeRquest{
    }

}
