var app = angular.module('app',['ngRoute'],function($httpProvider) {

  Date.prototype.Format = function (fmt) { //author: meizz 
    var o = {
        "M+": this.getMonth() + 1, //月份 
        "d+": this.getDate(), //日 
        "H+": this.getHours(), //小时 
        "m+": this.getMinutes(), //分 
        "s+": this.getSeconds(), //秒 
        "q+": Math.floor((this.getMonth() + 3) / 3), //季度 
        "S": this.getMilliseconds() //毫秒 
    };
    if (/(y+)/.test(fmt)) fmt = fmt.replace(RegExp.$1, (this.getFullYear() + "").substr(4 - RegExp.$1.length));
    for (var k in o)
    if (new RegExp("(" + k + ")").test(fmt)) fmt = fmt.replace(RegExp.$1, (RegExp.$1.length == 1) ? (o[k]) : (("00" + o[k]).substr(("" + o[k]).length)));
    return fmt;
  }

  // Use x-www-form-urlencoded Content-Type
  $httpProvider.defaults.headers.post['Content-Type'] = 'application/x-www-form-urlencoded;charset=utf-8';
 
  /**
   * The workhorse; converts an object to x-www-form-urlencoded serialization.
   * @param {Object} obj
   * @return {String}
   */ 
  var param = function(obj) {
    var query = '', name, value, fullSubName, subName, subValue, innerObj, i;
      
    for(name in obj) {
      value = obj[name];
        
      if(value instanceof Array) {
        for(i=0; i<value.length; ++i) {
          subValue = value[i];
          fullSubName = name + '[' + i + ']';
          innerObj = {};
          innerObj[fullSubName] = subValue;
          query += param(innerObj) + '&';
        }
      }
      else if(value instanceof Object) {
        for(subName in value) {
          subValue = value[subName];
          fullSubName = name + '[' + subName + ']';
          innerObj = {};
          innerObj[fullSubName] = subValue;
          query += param(innerObj) + '&';
        }
      }
      else if(value !== undefined && value !== null)
        query += encodeURIComponent(name) + '=' + encodeURIComponent(value) + '&';
    }
      
    return query.length ? query.substr(0, query.length - 1) : query;
  };
 
  // Override $http service's default transformRequest
  $httpProvider.defaults.transformRequest = [function(data) {
    return angular.isObject(data) && String(data) !== '[object File]' ? param(data) : data;
  }];
});

app.config(function($routeProvider){
  $routeProvider.when("/",{
    templateUrl:"/static/main.html",
    controller:"MainCtrl"
  });
  $routeProvider.when("/buysit/:oid",{
    templateUrl:"/static/buysit.html",
    controller:"BuySeatCtrl"
  });
  $routeProvider.when("/confirmation",{
    templateUrl:"/static/confirmation.html",
    controller:"ConfirmationCtrl"
  });
  $routeProvider.when("/buysitsuccess",{
    templateUrl:"/static/buysitsuccess.html",
    controller:"BuySuccessCtrl"
  });
  $routeProvider.when("/drivermyline",{
    templateUrl:"/static/drivermyline.html",
    controller:"DriverTripCtrl"
  });
  $routeProvider.when("/driverregister/:userType",{
    templateUrl:"/static/driverregister.html",
    controller:"DriverRegisterCtrl"
  });
  $routeProvider.when("/modifymycar",{
    templateUrl:"/static/modifymycar.html",
    controller:"ModifyCarCtrl"
  });
  $routeProvider.when("/mybill",{
    templateUrl:"/static/mybill.html",
    controller:"MyBillCtrl"
  });
  $routeProvider.when("/mycar",{
    templateUrl:"/static/mycar.html",
    controller:"MyCarCtrl"
  });
  $routeProvider.when("/passengermyline",{
    templateUrl:"/static/passengermyline.html",
    controller:"PassengerTripCtrl"
  });
  $routeProvider.when("/postline/:plateNumber",{
    templateUrl:"/static/postline.html",
    controller:"PublishTripCtrl"
  });
  $routeProvider.when("/profile",{
    templateUrl:"/static/profile.html",
    controller:"ProfileCtrl"
  });
  $routeProvider.when("/searchdetail/:lineId/:time",{
    templateUrl:"/static/searchdetail.html",
    controller:"SearchDetailCtrl"
  });

});
/*
app.service("LineService",['$http',function($http){
	var allLines = [];
	var hotLines = [];

	$http.post("/getLines",null).success(function(data){
		allLines = data;
	});
	$http.post("/getHotLines",null).success(function(data){
		hotLines = data;
	});
	this.getStarts = function() {
		var starts = [];
		for line in allLines {
			starts.push(line.start);
		}
		return starts;
	}
	this.getEnds = function(start) {
		var ends = [];
		for line in allLines {
			if(line.start == start) ends.push(line.end);	
		}
	}
}]);
*/
app.controller('BuySeatCtrl', ['$scope','$routeParams','$http','$location','$rootScope',function($scope,$routeParams,$http,$location,$rootScope){
  var oid = $routeParams.oid;
  $rootScope.oid = oid;
  $scope.count =1;
  var getTripDetail = function(oid) {
    $http.post("/tripDetail",{oid:oid}).success(function(trip){
      $scope.trip = trip;
    });
  }
  getTripDetail(oid);

  function pay(data) {
    
  }

  $scope.applyTrip = function() {
    $http.post("/applyTrip",{oid:oid,count:$scope.count}).success(function(data){
      if(data.success) {
        WeixinJSBridge.invoke(
          'getBrandWCPayRequest', 
          {
              "appId" :data.appId,     
               "timeStamp":data.timeStamp,         //时间戳，自1970年以来的秒数     
               "nonceStr":data.nonceStr, //随机串     
               "package":data.package,     
               "signType":"MD5",         //微信签名方式：     
               "paySign":data.paySign //微信签名 
          },
          function(res){     
             if(res.err_msg == "get_brand_wcpay_request:ok") {
                  $scope.$apply($location.url("/buysitsuccess"));
                  //window.location.href="index.html#/buysitsuccess";
             } else if(res.err_msg == "get_brand_wcpay_request:fail") {
                  alert("支付失败，请重试！");
             }   
          }
        ); 
      } else {
	      if(!data.enough) {
		      alert("剩余座位不够了:-)");
		      $location.url("/");
	      } else if(data.busy) {
		     alert("你正在参加拼车中");
		    $location.url("/passengermyline");
	      }	else if(data.noAuth){
	      	      $location.url("/confirmation");
	      } else {
		      alert("error");
	      }
      }
    });
  }

}]);

app.controller('ListCtrl', ['$scope','$location', function($scope,$location){
	$scope.buySeat = function(oid) {
		$location.url("/buysit/"+oid);
	}
}]);

app.controller('MainCtrl', ['$scope','$http','$location','$rootScope', function($scope,$http,$location,$rootScope){

  $scope.data = [];
  $scope.time = new Date();
  var userType = null;
  var plateNumber = null;
  var lineId = 1;

  $scope.allLines = {};
  $scope.ends = [];
  $scope.hotLines = [];
  $http.post("/getLines",null).success(function(data){
	angular.forEach(data,function(line,key) {
		if($scope.allLines[line.start]) {
			$scope.allLines[line.start].push(line);
		} else {
			$scope.allLines[line.start] = [line];
		}
		if(line.hot)
			$scope.hotLines.push(line);
	});
	$rootScope.allLines = $scope.allLines;
  });
  $scope.change = function(start) {
	$scope.ends = start;
  }

  $scope.search = function() {
	if($scope.end.id)
		lineId = $scope.end.id;
	$location.url("/searchdetail/"+lineId+"/"+$scope.time.getTime());
	
  }
  $http.post("/getTrips",null).success(function(data){
    $scope.data = data;
  });

  $http.post("/getUserInfo",null).success(function(data){
    if(data.login) {
      userType = data.userType;
      plateNumber = data.plateNumber || null;
    } 
  });
  $scope.publishTrip = function() {
    if(userType == "Owner") {
      $location.url("/postline/"+plateNumber);
    } else if (userType == null) {
      window.location.href="https://open.weixin.qq.com/connect/oauth2/authorize?appid=wxe9a0a490a170731d&redirect_uri=http%3A%2F%2Fgeekgogo.cn%2Fpinche%2Findex.html&response_type=code&scope=snsapi_userinfo&state=STATE#wechat_redirect";
    } else {
      $location.url("/driverregister/"+userType);
    }
  };

  $scope.toProfile = function() {
	  $location.url("/profile");
  }

}]);

app.controller('ConfirmationCtrl', ['$scope','$http','$location','$rootScope', function($scope,$http,$location,$rootScope){

  $scope.tel = "";
  $scope.code = "";
  $scope.submitBtn = true;
  $scope.getCodeBtn = false;

  $scope.getCode = function() {
    $http.post("/getCode",{tel:$scope.tel}).success(function(data){
      if(!data.success) {
        alert("get code error");
      } else {
        $scope.getCodeBtn = true;
        $scope.submitBtn = false; 
      }
    });
  };

  $scope.registerPassenger = function() {
    $http.post("/registerPassenger",{tel:$scope.tel,code:$scope.code}).success(function(data){
      if(data.success) {
        $location.url("/buysit/"+$rootScope.oid);
      } else {
        alert("registerPassenger error");
      }
    });
  };

}]);

app.controller('BuySuccessCtrl', ['$scope','$location', function($scope,$location){
  $scope.myTrip = function() {
    $location.url("/passengermyline");
  }
}]);

app.controller('DriverTripCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('PassengerTripCtrl', ['$scope','$location','$http', function($scope,$location,$http){
	$scope.isOwner = false;
	$scope.isPassenger = false;
	function loadData() {
	$http.post("/getTripInfo",null).success(function(data){
		if(!data.success) {
			$scope.isOwner = false;
			$scope.isPassenger = false;
			return ;
		}
		if(data.orders) {
			$scope.orders = data.orders;
			$scope.isOwner = true;
		} else {
			$scope.order = data.order;
			$scope.isPassenger = true;
		}
		$scope.trip = data.trip;
	});
	}
	loadData();
	$scope.submit = function() {
		$http.post("/submitOrder",null).success(function(data){
			loadData();
		});
	}

}]);

app.controller('DriverRegisterCtrl', ['$scope','$location','$routeParams','$http', function($scope,$location,$routeParams,$http){
  
  $scope.carType = "1";

  $scope.needAuth = false;
  $scope.submitBtn = true;
  $scope.getCodeBtn = false;
  
  if($routeParams.userType == "Anonymous") {
    $scope.needAuth = true;
  } else {
    $scope.needAuth = false;
    $scope.submitBtn = false;
  }

  $scope.getCode = function() {
    $http.post("/getCode",{tel:$scope.tel}).success(function(data){
      if(!data.success) {
        alert("get code error");
      } else {
        $scope.submitBtn = false;
        $scope.getCodeBtn = true;
      }
    });
  };

  $scope.becomeOwner = function() {
    $http.post("/registerOwner",{tel:$scope.tel,code:$scope.code,carType:$scope.carType,plateNumber:$scope.plateNumber}).success(function(data){
      if(data.success) {
        $location.url("/postline");
      } else {
        alert("auth code error");
      }
    });
  };


}]);

app.controller('ModifyCarCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('MyBillCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('MyCarCtrl', ['$scope','$location', function($scope,$location){

}]);

app.controller('PublishTripCtrl', ['$scope','$location','$http','$routeParams','$rootScope', function($scope,$location,$http,$routeParams,$rootScope){
  $scope.plateNumber = $routeParams.plateNumber;
  $scope.lineId = 1;
  $scope.ends = [];

  $scope.publishTrip = function() {
    var data = {lineId:$scope.line.id,startTime:$scope.startTime.Format("yyyy-MM-ddTHH:mm"),seatCount:$scope.seatCount,venue:$scope.venue};
    $http.post('/publishTrip',data).success(function(data){
      if(data.success) {
        $location.url("/passengermyline");
      } else if(data.busy){
	      alert("你正在参加拼车中");
	      $location.url("/passengermyline");
      } else {
        alert("publishTrip error");
      }
    });
  };
  $scope.changeStart = function(data) {
	$scope.ends = data;
  };

}]);

app.controller('ProfileCtrl', ['$scope','$location', function($scope,$location){
	$scope.toMyTrip = function() {
		$location.url("/passengermyline");
	}
}]);

app.controller('SearchDetailCtrl', ['$scope','$location','$routeParams','$http', function($scope,$location,$routeParams,$http){
	var time = new Date(parseInt($routeParams.time));
	var start = new Date(time.getFullYear(),time.getMonth(),time.getDate()).getTime()/1000;
	var end = new Date(time.getFullYear(),time.getMonth(),time.getDate()+1).getTime()/1000;
	$scope.data = [];
  	$http.post("/getTrips",null).success(function(data){
		angular.forEach(data,function(trip,key){
			if(trip.line_id == parseInt($routeParams.lineId) && trip.start_time > start && trip.start_time < end) {
				$scope.data.push(trip);
			}
		});
  	});

	$scope.toBuy = function(oid) {
		$location.url("/buysit/"+oid);
	}
}]);





