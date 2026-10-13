{
	tyObject_NodecolonObjectType___uOkeLMf1cjEjqM3LsA74rA* result;
NIM_BOOL* nimErr_;
{nimErr_ = nimErrorFlag();
	result = NIM_NIL;
	eqdestroy___optimized_u58(result);
	if (NIM_UNLIKELY(*nimErr_)) goto BeforeRet_;
	result = (tyObject_NodecolonObjectType___uOkeLMf1cjEjqM3LsA74rA*) nimNewObj(sizeof(tyObject_NodecolonObjectType___uOkeLMf1cjEjqM3LsA74rA), NIM_ALIGNOF(tyObject_NodecolonObjectType___uOkeLMf1cjEjqM3LsA74rA));
	{
		NI TM__dqdN7yeCbgZp3iqQ745zFg_2;
		tyObject_NodecolonObjectType___uOkeLMf1cjEjqM3LsA74rA* T5_;
		NI TM__dqdN7yeCbgZp3iqQ745zFg_3;
		tyObject_NodecolonObjectType___uOkeLMf1cjEjqM3LsA74rA* T6_;
		if (!(((NI)0) < depth_p0)) goto LA3_;
		if (nimSubInt(depth_p0, ((NI)1), &TM__dqdN7yeCbgZp3iqQ745zFg_2)) { raiseOverflow(); goto BeforeRet_;
		};
		T5_ = NIM_NIL;
		T5_ = make__optimized_u17((NI)(TM__dqdN7yeCbgZp3iqQ745zFg_2));
		if (NIM_UNLIKELY(*nimErr_)) {eqdestroy___optimized_u58(T5_); goto BeforeRet_;}
		eqsink___optimized_u67(&(*result).left, T5_);
		if (NIM_UNLIKELY(*nimErr_)) goto BeforeRet_;
		if (nimSubInt(depth_p0, ((NI)1), &TM__dqdN7yeCbgZp3iqQ745zFg_3)) { raiseOverflow(); goto BeforeRet_;
		};
		T6_ = NIM_NIL;
		T6_ = make__optimized_u17((NI)(TM__dqdN7yeCbgZp3iqQ745zFg_3));
		if (NIM_UNLIKELY(*nimErr_)) {eqdestroy___optimized_u58(T6_); goto BeforeRet_;}
		eqsink___optimized_u67(&(*result).right, T6_);
		if (NIM_UNLIKELY(*nimErr_)) goto BeforeRet_;
	}
LA3_: ;
	}BeforeRet_: ;
	return result;
}